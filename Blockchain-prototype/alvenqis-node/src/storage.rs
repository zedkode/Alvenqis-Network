use crate::error::{NodeError, NodeResult};
use alvenqis_core::{cumulative_work, hash_to_hex, Block};
use fs2::FileExt;
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, MAIN_DB,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const CHAIN_DATABASE_FILE_NAME: &str = "chain.sqlite3";
pub const LEGACY_CHAIN_FILE_NAME: &str = "chain.jsonl";
/// Compatibility alias for callers that only need the canonical storage path.
pub const CHAIN_FILE_NAME: &str = CHAIN_DATABASE_FILE_NAME;
pub const CHAIN_LOCK_FILE_NAME: &str = "chain.lock";

const STORAGE_SCHEMA_VERSION: i64 = 1;
const STORAGE_APPLICATION_ID: i64 = 0x5649_5245; // "ALVE"
const MINIMUM_SAFE_SQLITE_VERSION: i32 = 3_051_003;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

type ValidatedBlockHashes = BTreeMap<i64, Vec<u8>>;
type ValidatedBlockHashCache = BTreeMap<PathBuf, ValidatedBlockHashes>;

static VALIDATED_BLOCK_HASH_CACHE: OnceLock<Mutex<ValidatedBlockHashCache>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FileFingerprint {
    pub len: u64,
    pub modified_millis: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChainStorageFingerprint {
    pub database: FileFingerprint,
    pub wal: FileFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredChainIdentity {
    pub genesis_hash: String,
    pub best_height: u64,
    pub best_hash: String,
    pub cumulative_work: String,
}

pub trait BlockStore {
    fn load_blocks(&self) -> NodeResult<Vec<Block>>;

    fn append_validated<R, F>(&self, candidate: &Block, validate: F) -> NodeResult<R>
    where
        F: FnOnce(&[Block], &Block) -> NodeResult<R>;

    fn replace_validated<R, F>(
        &self,
        expected_tip: &str,
        candidate: &[Block],
        validate: F,
    ) -> NodeResult<R>
    where
        F: FnOnce(&[Block], &[Block]) -> NodeResult<R>;
}

#[derive(Clone, Debug)]
pub struct SqliteBlockStore {
    data_dir: PathBuf,
}

impl SqliteBlockStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    fn lock_file_path(&self) -> PathBuf {
        self.data_dir.join(CHAIN_LOCK_FILE_NAME)
    }

    fn open_exclusive_lock(&self) -> NodeResult<File> {
        ensure_data_dir(&self.data_dir)?;
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(self.lock_file_path())?;
        FileExt::lock_exclusive(&lock_file)?;
        Ok(lock_file)
    }

    fn prepare_database(&self, create_if_missing: bool) -> NodeResult<()> {
        ensure_data_dir(&self.data_dir)?;
        let database_path = chain_database_path(&self.data_dir);
        if database_path.exists() {
            return Ok(());
        }

        let _lock = self.open_exclusive_lock()?;
        if database_path.exists() {
            return Ok(());
        }

        let legacy_path = legacy_chain_file_path(&self.data_dir);
        let legacy_blocks = if legacy_path.exists() {
            Some(load_legacy_blocks_from_path(&legacy_path)?)
        } else {
            None
        };
        if legacy_blocks.is_none() && !create_if_missing {
            return Err(NodeError::ChainNotInitialized(database_path));
        }

        let temporary_path = self.data_dir.join(format!(
            ".{CHAIN_DATABASE_FILE_NAME}.migrating-{}",
            std::process::id()
        ));
        if temporary_path.exists() {
            fs::remove_file(&temporary_path)?;
        }

        let create_result = (|| -> NodeResult<()> {
            let mut connection = Connection::open(&temporary_path)?;
            configure_connection(&connection, false)?;
            initialize_schema(&connection)?;
            if let Some(blocks) = legacy_blocks.as_deref() {
                let transaction =
                    connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                for block in blocks {
                    insert_canonical_block(&transaction, block)?;
                }
                set_metadata(&transaction, "migration_source", LEGACY_CHAIN_FILE_NAME)?;
                set_metadata(
                    &transaction,
                    "migration_completed_unix_seconds",
                    &unix_seconds().to_string(),
                )?;
                transaction.commit()?;
            }
            verify_database_integrity_connection(&connection, &temporary_path)?;
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "FULL")?;
            drop(connection);
            fs::rename(&temporary_path, &database_path)?;
            sync_parent_directory(&self.data_dir)?;
            Ok(())
        })();

        if create_result.is_err() && temporary_path.exists() {
            let _ = fs::remove_file(&temporary_path);
        }
        create_result
    }

    fn open_read_connection(&self) -> NodeResult<Connection> {
        self.prepare_database(false)?;
        let connection = Connection::open_with_flags(
            chain_database_path(&self.data_dir),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_read_connection(&connection)?;
        validate_schema(&connection, &chain_database_path(&self.data_dir))?;
        Ok(connection)
    }

    fn open_write_connection(&self) -> NodeResult<Connection> {
        self.prepare_database(true)?;
        let connection = Connection::open_with_flags(
            chain_database_path(&self.data_dir),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_connection(&connection, true)?;
        validate_schema(&connection, &chain_database_path(&self.data_dir))?;
        Ok(connection)
    }

    pub fn load_tip_block(&self) -> NodeResult<Option<Block>> {
        let connection = self.open_read_connection()?;
        load_tip_block_from_connection(&connection, &chain_database_path(&self.data_dir))
    }

    pub fn load_blocks_from_height(&self, start_height: u64) -> NodeResult<Vec<Block>> {
        let connection = self.open_read_connection()?;
        load_blocks_from_height_from_connection(
            &connection,
            &chain_database_path(&self.data_dir),
            start_height,
        )
    }

    pub fn canonical_block_count(&self) -> NodeResult<u64> {
        let connection = self.open_read_connection()?;
        canonical_block_count_from_connection(&connection, &chain_database_path(&self.data_dir))
    }

    fn append_with_tip_link(&self, block: &Block) -> NodeResult<()> {
        let mut connection = self.open_write_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let tip =
            load_tip_block_from_connection(&transaction, &chain_database_path(&self.data_dir))?;
        verify_tip_extension(tip.as_ref(), block)?;
        insert_canonical_block(&transaction, block)?;
        transaction.commit()?;
        Ok(())
    }

    /// Test/bootstrap helper that deliberately skips structural checks.
    fn append_unchecked(&self, block: &Block) -> NodeResult<()> {
        let mut connection = self.open_write_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_canonical_block(&transaction, block)?;
        transaction.commit()?;
        Ok(())
    }
}

impl BlockStore for SqliteBlockStore {
    fn load_blocks(&self) -> NodeResult<Vec<Block>> {
        let connection = self.open_read_connection()?;
        load_blocks_from_connection(&connection, &chain_database_path(&self.data_dir), false)
    }

    fn append_validated<R, F>(&self, candidate: &Block, validate: F) -> NodeResult<R>
    where
        F: FnOnce(&[Block], &Block) -> NodeResult<R>,
    {
        let mut connection = self.open_write_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let blocks =
            load_blocks_from_connection(&transaction, &chain_database_path(&self.data_dir), false)?;
        let result = validate(&blocks, candidate)?;
        verify_tip_extension(blocks.last(), candidate)?;
        insert_canonical_block(&transaction, candidate)?;
        transaction.commit()?;
        Ok(result)
    }

    fn replace_validated<R, F>(
        &self,
        expected_tip: &str,
        candidate: &[Block],
        validate: F,
    ) -> NodeResult<R>
    where
        F: FnOnce(&[Block], &[Block]) -> NodeResult<R>,
    {
        let mut connection = self.open_write_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current =
            load_blocks_from_connection(&transaction, &chain_database_path(&self.data_dir), false)?;
        let actual_tip = match current.last() {
            Some(block) => hash_to_hex(&block.hash()?),
            None => "none".to_owned(),
        };
        if actual_tip != expected_tip {
            return Err(NodeError::StaleChainTip {
                expected: expected_tip.to_owned(),
                actual: actual_tip,
            });
        }

        let result = validate(&current, candidate)?;
        replace_canonical_chain(&transaction, &current, candidate, expected_tip)?;
        transaction.commit()?;
        Ok(result)
    }
}

fn verify_tip_extension(tip: Option<&Block>, block: &Block) -> NodeResult<()> {
    if let Some(tip) = tip {
        let expected_tip = hash_to_hex(&tip.hash()?);
        let actual_previous = hash_to_hex(&block.header.previous_hash);
        if expected_tip != actual_previous {
            return Err(NodeError::StaleChainTip {
                expected: expected_tip,
                actual: actual_previous,
            });
        }
        let expected_height = tip.header.height.saturating_add(1);
        if block.header.height != expected_height {
            return Err(NodeError::Input(format!(
                "block height {} does not extend tip height {} (expected {})",
                block.header.height, tip.header.height, expected_height
            )));
        }
    } else if block.header.height != 0 {
        return Err(NodeError::Input(format!(
            "first chain block must be height 0, got {}",
            block.header.height
        )));
    }
    Ok(())
}

fn replace_canonical_chain(
    transaction: &Transaction<'_>,
    current: &[Block],
    candidate: &[Block],
    source_tip_hash: &str,
) -> NodeResult<()> {
    let mut candidate_hashes = BTreeSet::new();
    for block in candidate {
        candidate_hashes.insert(*block.hash()?.as_bytes());
    }
    for block in current {
        if !candidate_hashes.contains(block.hash()?.as_bytes()) {
            insert_orphaned_block(transaction, block, source_tip_hash)?;
        }
    }

    transaction.execute("DELETE FROM canonical_blocks", [])?;
    for block in candidate {
        insert_canonical_block(transaction, block)?;
    }
    Ok(())
}

fn insert_canonical_block(connection: &Connection, block: &Block) -> NodeResult<()> {
    let height = height_to_i64(block.header.height)?;
    let hash = block.hash()?;
    let block_json = serde_json::to_vec(block)?;
    connection.execute(
        "INSERT INTO canonical_blocks
         (height, hash, previous_hash, network_id, block_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            height,
            hash.as_bytes().as_slice(),
            block.header.previous_hash.as_bytes().as_slice(),
            block.header.network_id,
            block_json
        ],
    )?;
    Ok(())
}

fn insert_orphaned_block(
    connection: &Connection,
    block: &Block,
    source_tip_hash: &str,
) -> NodeResult<()> {
    let height = height_to_i64(block.header.height)?;
    let hash = block.hash()?;
    let block_json = serde_json::to_vec(block)?;
    connection.execute(
        "INSERT INTO orphaned_blocks
         (hash, height, previous_hash, network_id, block_json, detached_at_unix_seconds, source_tip_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(hash) DO UPDATE SET
           detached_at_unix_seconds = excluded.detached_at_unix_seconds,
           source_tip_hash = excluded.source_tip_hash",
        params![
            hash.as_bytes().as_slice(),
            height,
            block.header.previous_hash.as_bytes().as_slice(),
            block.header.network_id,
            block_json,
            unix_seconds() as i64,
            source_tip_hash
        ],
    )?;
    Ok(())
}

#[derive(Debug)]
struct StoredCanonicalBlockRow {
    height: i64,
    hash: Vec<u8>,
    previous_hash: Vec<u8>,
    network_id: String,
    block_json: Vec<u8>,
}

#[derive(Debug)]
struct ValidatedCanonicalBlockRow {
    height: i64,
    hash: Vec<u8>,
    previous_hash: Vec<u8>,
    block: Block,
}

fn stored_canonical_block_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredCanonicalBlockRow> {
    Ok(StoredCanonicalBlockRow {
        height: row.get(0)?,
        hash: row.get(1)?,
        previous_hash: row.get(2)?,
        network_id: row.get(3)?,
        block_json: row.get(4)?,
    })
}

fn validate_stored_canonical_block_row(
    stored: StoredCanonicalBlockRow,
    database_path: &Path,
    hash_was_validated: bool,
) -> NodeResult<ValidatedCanonicalBlockRow> {
    let StoredCanonicalBlockRow {
        height,
        hash,
        previous_hash,
        network_id,
        block_json,
    } = stored;
    if height < 0 || hash.len() != 32 || previous_hash.len() != 32 {
        return Err(invalid_database(
            database_path,
            format!("invalid canonical block columns at height {height}"),
        ));
    }
    let block: Block = serde_json::from_slice(&block_json).map_err(|error| {
        invalid_database(
            database_path,
            format!("cannot decode block at height {height}: {error}"),
        )
    })?;
    let block_height = height_to_i64(block.header.height)?;
    let hash_matches = hash_was_validated || hash.as_slice() == block.hash()?.as_bytes();
    if block_height != height
        || !hash_matches
        || previous_hash.as_slice() != block.header.previous_hash.as_bytes()
        || network_id != block.header.network_id
    {
        return Err(invalid_database(
            database_path,
            format!("stored columns do not match serialized block at height {height}"),
        ));
    }
    Ok(ValidatedCanonicalBlockRow {
        height,
        hash,
        previous_hash,
        block,
    })
}

fn load_validated_canonical_rows_from_height(
    connection: &Connection,
    database_path: &Path,
    start_height: i64,
    cached_hashes: &BTreeMap<i64, Vec<u8>>,
) -> NodeResult<Vec<ValidatedCanonicalBlockRow>> {
    let mut statement = connection.prepare(
        "SELECT height, hash, previous_hash, network_id, block_json
         FROM canonical_blocks
         WHERE height >= ?1
         ORDER BY height ASC",
    )?;
    let rows = statement.query_map(params![start_height], stored_canonical_block_row)?;
    let mut previous_stored_hash: Option<Vec<u8>> = None;
    let mut expected_height = start_height;
    let mut validated_rows = Vec::new();
    for row in rows {
        let stored = row?;
        let hash_was_validated = cached_hashes
            .get(&stored.height)
            .is_some_and(|cached| cached == &stored.hash);
        let validated =
            validate_stored_canonical_block_row(stored, database_path, hash_was_validated)?;
        if validated.height != expected_height {
            return Err(invalid_database(
                database_path,
                format!(
                    "non-contiguous canonical height: expected {expected_height}, found {}",
                    validated.height
                ),
            ));
        }
        if previous_stored_hash
            .as_deref()
            .is_some_and(|previous| previous != validated.previous_hash.as_slice())
        {
            return Err(NodeError::InvalidChainFile {
                path: database_path.to_path_buf(),
                line: validated.height as usize + 1,
                message: format!("broken previous_hash link at height {}", validated.height),
            });
        }
        expected_height = expected_height.saturating_add(1);
        previous_stored_hash = Some(validated.hash.clone());
        validated_rows.push(validated);
    }
    Ok(validated_rows)
}

fn load_tip_block_from_connection(
    connection: &Connection,
    database_path: &Path,
) -> NodeResult<Option<Block>> {
    let stored = connection
        .query_row(
            "SELECT height, hash, previous_hash, network_id, block_json
             FROM canonical_blocks
             ORDER BY height DESC
             LIMIT 1",
            [],
            stored_canonical_block_row,
        )
        .optional()?;
    stored
        .map(|stored| {
            validate_stored_canonical_block_row(stored, database_path, false)
                .map(|validated| validated.block)
        })
        .transpose()
}

fn load_blocks_from_height_from_connection(
    connection: &Connection,
    database_path: &Path,
    start_height: u64,
) -> NodeResult<Vec<Block>> {
    let start_height = height_to_i64(start_height)?;
    let query_start_height = if start_height == 0 {
        0
    } else {
        start_height - 1
    };
    let rows = load_validated_canonical_rows_from_height(
        connection,
        database_path,
        query_start_height,
        &BTreeMap::new(),
    )?;
    Ok(rows
        .into_iter()
        .filter(|row| row.height >= start_height)
        .map(|row| row.block)
        .collect())
}

fn canonical_block_count_from_connection(
    connection: &Connection,
    database_path: &Path,
) -> NodeResult<u64> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM canonical_blocks", [], |row| {
        row.get(0)
    })?;
    u64::try_from(count).map_err(|_| {
        invalid_database(
            database_path,
            format!("invalid canonical block count {count}"),
        )
    })
}

fn load_blocks_from_connection(
    connection: &Connection,
    database_path: &Path,
    allow_empty: bool,
) -> NodeResult<Vec<Block>> {
    let validation_cache = VALIDATED_BLOCK_HASH_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let cached_hashes = validation_cache
        .lock()
        .map_err(|_| NodeError::Input("block validation cache lock poisoned".to_owned()))?
        .get(database_path)
        .cloned()
        .unwrap_or_default();
    let rows =
        load_validated_canonical_rows_from_height(connection, database_path, 0, &cached_hashes)?;
    let mut validated_hashes = BTreeMap::new();
    let mut blocks = Vec::with_capacity(rows.len());
    for row in rows {
        validated_hashes.insert(row.height, row.hash);
        blocks.push(row.block);
    }

    if blocks.is_empty() && !allow_empty {
        return Err(NodeError::ChainNotInitialized(database_path.to_path_buf()));
    }
    validation_cache
        .lock()
        .map_err(|_| NodeError::Input("block validation cache lock poisoned".to_owned()))?
        .insert(database_path.to_path_buf(), validated_hashes);
    Ok(blocks)
}

fn configure_read_connection(connection: &Connection) -> NodeResult<()> {
    verify_sqlite_runtime()?;
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "query_only", true)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    Ok(())
}

fn configure_connection(connection: &Connection, enable_wal: bool) -> NodeResult<()> {
    verify_sqlite_runtime()?;
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    if enable_wal {
        connection.pragma_update(None, "journal_mode", "WAL")?;
    } else {
        connection.pragma_update(None, "journal_mode", "DELETE")?;
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    connection.pragma_update(None, "wal_autocheckpoint", 1_000_i64)?;
    Ok(())
}

fn verify_sqlite_runtime() -> NodeResult<()> {
    let version = rusqlite::version_number();
    if version < MINIMUM_SAFE_SQLITE_VERSION {
        return Err(NodeError::Input(format!(
            "SQLite {} is below required safe version 3.51.3",
            rusqlite::version()
        )));
    }
    Ok(())
}

fn initialize_schema(connection: &Connection) -> NodeResult<()> {
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         CREATE TABLE storage_metadata (
           key TEXT PRIMARY KEY NOT NULL,
           value TEXT NOT NULL
         ) STRICT;
         CREATE TABLE canonical_blocks (
           height INTEGER PRIMARY KEY NOT NULL CHECK(height >= 0),
           hash BLOB NOT NULL UNIQUE CHECK(length(hash) = 32),
           previous_hash BLOB NOT NULL CHECK(length(previous_hash) = 32),
           network_id TEXT NOT NULL,
           block_json BLOB NOT NULL CHECK(length(block_json) > 0)
         ) STRICT;
         CREATE TABLE orphaned_blocks (
           hash BLOB PRIMARY KEY NOT NULL CHECK(length(hash) = 32),
           height INTEGER NOT NULL CHECK(height >= 0),
           previous_hash BLOB NOT NULL CHECK(length(previous_hash) = 32),
           network_id TEXT NOT NULL,
           block_json BLOB NOT NULL CHECK(length(block_json) > 0),
           detached_at_unix_seconds INTEGER NOT NULL,
           source_tip_hash TEXT NOT NULL
         ) STRICT;
         CREATE INDEX orphaned_blocks_height_idx ON orphaned_blocks(height);
         PRAGMA application_id = 1447645765;
         PRAGMA user_version = 1;
         INSERT INTO storage_metadata(key, value) VALUES ('schema_version', '1');
         INSERT INTO storage_metadata(key, value) VALUES ('backend', 'sqlite');
         COMMIT;",
    )?;
    validate_schema(connection, Path::new(CHAIN_DATABASE_FILE_NAME))
}

fn validate_schema(connection: &Connection, database_path: &Path) -> NodeResult<()> {
    let application_id: i64 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let user_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let metadata_version: Option<String> = connection
        .query_row(
            "SELECT value FROM storage_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if application_id != STORAGE_APPLICATION_ID
        || user_version != STORAGE_SCHEMA_VERSION
        || metadata_version.as_deref() != Some("1")
    {
        return Err(invalid_database(
            database_path,
            format!(
                "unsupported schema identity: application_id={application_id}, user_version={user_version}, metadata_version={metadata_version:?}"
            ),
        ));
    }
    Ok(())
}

fn set_metadata(connection: &Connection, key: &str, value: &str) -> NodeResult<()> {
    connection.execute(
        "INSERT INTO storage_metadata(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn height_to_i64(height: u64) -> NodeResult<i64> {
    i64::try_from(height)
        .map_err(|_| NodeError::Input(format!("block height {height} exceeds SQLite range")))
}

fn invalid_database(path: &Path, message: String) -> NodeError {
    NodeError::InvalidChainDatabase {
        path: path.to_path_buf(),
        message,
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn file_fingerprint(path: &Path) -> FileFingerprint {
    fs::metadata(path).map_or_else(
        |_| FileFingerprint::default(),
        |metadata| FileFingerprint {
            len: metadata.len(),
            modified_millis: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
                .unwrap_or(0),
        },
    )
}

fn sync_parent_directory(path: &Path) -> NodeResult<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn chain_database_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CHAIN_DATABASE_FILE_NAME)
}

/// Compatibility accessor. New callers should use [`chain_database_path`].
pub fn chain_file_path(data_dir: &Path) -> PathBuf {
    chain_database_path(data_dir)
}

pub fn legacy_chain_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LEGACY_CHAIN_FILE_NAME)
}

pub fn chain_storage_fingerprint(data_dir: &Path) -> ChainStorageFingerprint {
    let database_path = chain_database_path(data_dir);
    let mut wal_path = database_path.as_os_str().to_os_string();
    wal_path.push("-wal");
    let wal_path = PathBuf::from(wal_path);
    ChainStorageFingerprint {
        database: file_fingerprint(&database_path),
        wal: file_fingerprint(&wal_path),
    }
}

pub fn chain_storage_exists(data_dir: &Path) -> bool {
    chain_database_path(data_dir).exists() || legacy_chain_file_path(data_dir).exists()
}

pub fn ensure_data_dir(data_dir: &Path) -> NodeResult<()> {
    fs::create_dir_all(data_dir)?;
    Ok(())
}

pub fn append_block(data_dir: &Path, block: &Block) -> NodeResult<()> {
    SqliteBlockStore::new(data_dir).append_with_tip_link(block)
}

/// Unchecked append for intentional invalid-chain fixtures in tests only.
pub fn append_block_unchecked(data_dir: &Path, block: &Block) -> NodeResult<()> {
    SqliteBlockStore::new(data_dir).append_unchecked(block)
}

pub fn load_blocks(data_dir: &Path) -> NodeResult<Vec<Block>> {
    SqliteBlockStore::new(data_dir).load_blocks()
}

pub fn load_stored_chain_identity(data_dir: &Path) -> NodeResult<StoredChainIdentity> {
    let store = SqliteBlockStore::new(data_dir);
    let connection = store.open_read_connection()?;
    let database_path = chain_database_path(data_dir);
    let mut statement = connection.prepare(
        "SELECT height, hash, previous_hash, network_id, block_json
         FROM canonical_blocks ORDER BY height ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Vec<u8>>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Vec<u8>>(4)?,
        ))
    })?;

    let mut blocks = Vec::new();
    let mut genesis_hash = None;
    let mut best_hash = None;
    let mut previous_stored_hash: Option<Vec<u8>> = None;
    for row in rows {
        let (height, stored_hash, stored_previous, stored_network, block_json) = row?;
        if height < 0 || stored_hash.len() != 32 || stored_previous.len() != 32 {
            return Err(invalid_database(
                &database_path,
                format!("invalid stored chain identity at height {height}"),
            ));
        }
        let block: Block = serde_json::from_slice(&block_json).map_err(|error| {
            invalid_database(
                &database_path,
                format!("cannot decode block at height {height}: {error}"),
            )
        })?;
        if block.header.height != height as u64
            || block.header.previous_hash.as_bytes() != stored_previous.as_slice()
            || block.header.network_id != stored_network
            || previous_stored_hash
                .as_deref()
                .is_some_and(|previous| previous != stored_previous.as_slice())
        {
            return Err(invalid_database(
                &database_path,
                format!("stored chain identity mismatch at height {height}"),
            ));
        }
        let stored_hash_hex = stored_hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        genesis_hash.get_or_insert_with(|| stored_hash_hex.clone());
        best_hash = Some(stored_hash_hex);
        previous_stored_hash = Some(stored_hash);
        blocks.push(block);
    }

    let tip = blocks
        .last()
        .ok_or_else(|| NodeError::ChainNotInitialized(database_path.clone()))?;
    Ok(StoredChainIdentity {
        genesis_hash: genesis_hash.expect("non-empty chain has genesis hash"),
        best_height: tip.header.height,
        best_hash: best_hash.expect("non-empty chain has tip hash"),
        cumulative_work: cumulative_work(&blocks)?.to_string(),
    })
}

pub fn backup_chain_database(data_dir: &Path, destination: &Path) -> NodeResult<()> {
    let store = SqliteBlockStore::new(data_dir);
    let source = store.open_read_connection()?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        return Err(NodeError::Input(format!(
            "backup destination already exists: {}",
            destination.display()
        )));
    }
    source.backup(MAIN_DB, destination, None)?;
    let backup = Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    validate_schema(&backup, destination)?;
    verify_database_integrity_connection(&backup, destination)?;
    Ok(())
}

pub fn verify_database_integrity(data_dir: &Path) -> NodeResult<()> {
    let store = SqliteBlockStore::new(data_dir);
    let connection = store.open_read_connection()?;
    verify_database_integrity_connection(&connection, &chain_database_path(data_dir))
}

fn verify_database_integrity_connection(
    connection: &Connection,
    database_path: &Path,
) -> NodeResult<()> {
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(invalid_database(
            database_path,
            format!("SQLite integrity_check failed: {result}"),
        ));
    }
    Ok(())
}

fn load_legacy_blocks_from_path(chain_path: &Path) -> NodeResult<Vec<Block>> {
    let file = File::open(chain_path)?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let block =
            serde_json::from_str::<Block>(&line).map_err(|error| NodeError::InvalidChainFile {
                path: chain_path.to_path_buf(),
                line: index + 1,
                message: error.to_string(),
            })?;
        if alvenqis_core::Network::from_network_id(&block.header.network_id).is_none() {
            return Err(NodeError::InvalidChainFile {
                path: chain_path.to_path_buf(),
                line: index + 1,
                message: format!(
                    "unknown or unsupported network_id '{}'",
                    block.header.network_id
                ),
            });
        }
        blocks.push(block);
    }

    if blocks.is_empty() {
        return Err(NodeError::ChainNotInitialized(chain_path.to_path_buf()));
    }
    verify_chain_structure(chain_path, &blocks)?;
    Ok(blocks)
}

pub fn verify_chain_structure(chain_path: &Path, blocks: &[Block]) -> NodeResult<()> {
    if blocks.is_empty() {
        return Ok(());
    }
    if blocks[0].header.height != 0 {
        return Err(NodeError::InvalidChainFile {
            path: chain_path.to_path_buf(),
            line: 1,
            message: format!(
                "genesis height must be 0, found {}",
                blocks[0].header.height
            ),
        });
    }
    for index in 1..blocks.len() {
        let previous = &blocks[index - 1];
        let block = &blocks[index];
        let line = index + 1;
        let expected_height = previous.header.height.saturating_add(1);
        if block.header.height != expected_height {
            return Err(NodeError::InvalidChainFile {
                path: chain_path.to_path_buf(),
                line,
                message: format!(
                    "non-contiguous height: expected {expected_height}, found {}",
                    block.header.height
                ),
            });
        }
        let expected_previous = hash_to_hex(&previous.hash()?);
        let actual_previous = hash_to_hex(&block.header.previous_hash);
        if expected_previous != actual_previous {
            return Err(NodeError::InvalidChainFile {
                path: chain_path.to_path_buf(),
                line,
                message: format!(
                    "broken previous_hash link: expected {expected_previous}, found {actual_previous}"
                ),
            });
        }
    }
    Ok(())
}

pub fn reset_data_dir(data_dir: &Path) -> NodeResult<()> {
    if data_dir.exists() {
        fs::remove_dir_all(data_dir)?;
    }
    fs::create_dir_all(data_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_core::{devnet_genesis, Address, PrivateKey};

    fn miner_address() -> String {
        Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            alvenqis_core::Network::Devnet,
        )
        .to_string()
    }

    fn linked_child(genesis: &Block) -> Block {
        let mut child = genesis.clone();
        child.header.height = genesis.header.height.saturating_add(1);
        child.header.previous_hash = genesis.hash().expect("parent hash");
        child
    }

    #[test]
    fn bundled_sqlite_contains_wal_reset_fix() {
        assert!(rusqlite::version_number() >= MINIMUM_SAFE_SQLITE_VERSION);
    }

    #[test]
    fn tip_append_persists_transactionally() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");
        store
            .append_with_tip_link(&linked_child(&genesis))
            .expect("child");
        let loaded = store.load_blocks().expect("load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].header.height, 0);
        assert_eq!(loaded[1].header.height, 1);
        let identity = load_stored_chain_identity(dir.path()).expect("stored identity");
        assert_eq!(
            identity.genesis_hash,
            hash_to_hex(&loaded[0].hash().expect("hash"))
        );
        assert_eq!(identity.best_height, 1);
        assert_eq!(
            identity.best_hash,
            hash_to_hex(&loaded[1].hash().expect("hash"))
        );
        assert_eq!(
            identity.cumulative_work,
            cumulative_work(&loaded).expect("work").to_string()
        );
        verify_database_integrity(dir.path()).expect("integrity");
    }

    #[test]
    fn point_queries_return_validated_tip_and_count() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");

        assert_eq!(store.canonical_block_count().expect("count"), 2);
        assert_eq!(store.load_tip_block().expect("tip"), Some(child.clone()));

        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_blocks
                 SET network_id = 'tampered-network'
                 WHERE height = 1",
                [],
            )
            .expect("tamper tip");
        let error = store.load_tip_block().expect_err("tip must be validated");
        assert!(matches!(error, NodeError::InvalidChainDatabase { .. }));
    }

    #[test]
    fn range_query_returns_validated_suffix() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        let grandchild = linked_child(&child);
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");
        store.append_with_tip_link(&grandchild).expect("grandchild");

        assert_eq!(
            store.load_blocks_from_height(1).expect("suffix"),
            vec![child.clone(), grandchild.clone()]
        );
        assert_eq!(
            store.load_blocks_from_height(2).expect("tail"),
            vec![grandchild]
        );
        assert!(store
            .load_blocks_from_height(3)
            .expect("empty suffix")
            .is_empty());

        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_blocks
                 SET previous_hash = zeroblob(32)
                 WHERE height = 2",
                [],
            )
            .expect("tamper range");
        let error = store
            .load_blocks_from_height(1)
            .expect_err("range must be validated");
        assert!(matches!(error, NodeError::InvalidChainDatabase { .. }));
    }

    #[test]
    fn tip_append_rejects_stale_parent_without_mutation() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");
        let mut stale_child = linked_child(&genesis);
        stale_child.header.previous_hash = alvenqis_core::Hash::zero();

        let error = store
            .append_with_tip_link(&stale_child)
            .expect_err("stale parent must fail");
        assert!(matches!(error, NodeError::StaleChainTip { .. }));
        assert_eq!(store.canonical_block_count().expect("count"), 1);
        assert_eq!(store.load_tip_block().expect("tip"), Some(genesis));
    }

    #[test]
    fn replace_validated_archives_detached_block() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");
        let tip = hash_to_hex(&child.hash().expect("child hash"));
        store
            .replace_validated(
                &tip,
                std::slice::from_ref(&genesis),
                |_current, _candidate| Ok(()),
            )
            .expect("replace");
        assert_eq!(store.load_blocks().expect("load"), vec![genesis]);
        let connection = store.open_read_connection().expect("open");
        let orphan_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM orphaned_blocks", [], |row| row.get(0))
            .expect("count");
        assert_eq!(orphan_count, 1);
    }

    #[test]
    fn load_rejects_broken_previous_hash_link() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");
        let mut orphan = linked_child(&genesis);
        orphan.header.previous_hash = alvenqis_core::Hash::zero();
        store.append_unchecked(&orphan).expect("corrupt fixture");
        let error = store.load_blocks().expect_err("must reject broken link");
        assert!(matches!(error, NodeError::InvalidChainFile { .. }));
    }

    #[test]
    fn migrates_legacy_jsonl_without_modifying_it() {
        let dir = tempfile::tempdir().expect("temp");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let legacy_path = legacy_chain_file_path(dir.path());
        let original = format!("{}\n", serde_json::to_string(&genesis).expect("json"));
        fs::write(&legacy_path, &original).expect("legacy");

        let loaded = load_blocks(dir.path()).expect("migrate and load");
        assert_eq!(loaded, vec![genesis]);
        assert!(chain_database_path(dir.path()).exists());
        assert_eq!(
            fs::read_to_string(legacy_path).expect("legacy remains"),
            original
        );
    }

    #[test]
    fn online_backup_is_valid_and_complete() {
        let dir = tempfile::tempdir().expect("temp");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        append_block(dir.path(), &genesis).expect("append");
        let backup_path = dir.path().join("backups").join(CHAIN_DATABASE_FILE_NAME);
        backup_chain_database(dir.path(), &backup_path).expect("backup");
        let backup_dir = backup_path.parent().expect("parent");
        assert_eq!(load_blocks(backup_dir).expect("backup load"), vec![genesis]);
    }

    #[test]
    fn wal_allows_a_reader_while_the_node_appends() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");

        let reader = store.open_read_connection().expect("reader");
        let before: i64 = reader
            .query_row("SELECT COUNT(*) FROM canonical_blocks", [], |row| {
                row.get(0)
            })
            .expect("before count");
        assert_eq!(before, 1);

        store
            .append_with_tip_link(&linked_child(&genesis))
            .expect("append while reader is open");
        let after: i64 = reader
            .query_row("SELECT COUNT(*) FROM canonical_blocks", [], |row| {
                row.get(0)
            })
            .expect("after count");
        assert_eq!(after, 2);
    }

    /// Truncated / partial last line in legacy chain.jsonl must fail migration
    /// without leaving a partial chain.sqlite3 behind.
    #[test]
    fn legacy_jsonl_truncated_line_fails_migration_cleanly() {
        let dir = tempfile::tempdir().expect("temp");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let legacy_path = legacy_chain_file_path(dir.path());
        let full = serde_json::to_string(&genesis).expect("json");
        // Partial last line (cut mid-JSON).
        let truncated = &full[..full.len().saturating_sub(full.len() / 3).max(1)];
        fs::write(&legacy_path, truncated).expect("write truncated");

        let error = load_blocks(dir.path()).expect_err("truncated jsonl must fail");
        assert!(
            matches!(
                error,
                NodeError::InvalidChainFile { .. } | NodeError::Io(_) | NodeError::Json(_)
            ) || error.to_string().to_lowercase().contains("json")
                || error.to_string().to_lowercase().contains("chain")
                || error.to_string().to_lowercase().contains("legacy"),
            "unexpected error kind: {error:?}"
        );
        assert!(
            !chain_database_path(dir.path()).exists(),
            "failed migration must not leave chain.sqlite3"
        );
        // Migrating temp must be cleaned up.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("migrating") || name.ends_with(".sqlite3"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "leftover migration artifacts: {leftovers:?}"
        );
    }

    /// Valid JSON but wrong network_id on legacy line must fail without a corrupt DB.
    #[test]
    fn legacy_jsonl_wrong_network_id_fails_migration_cleanly() {
        let dir = tempfile::tempdir().expect("temp");
        let mut genesis = devnet_genesis(&miner_address()).expect("genesis");
        genesis.header.network_id = "not-a-real-network".to_owned();
        let legacy_path = legacy_chain_file_path(dir.path());
        fs::write(
            &legacy_path,
            format!("{}\n", serde_json::to_string(&genesis).expect("json")),
        )
        .expect("write");

        let error = load_blocks(dir.path()).expect_err("bad network must fail");
        assert!(
            !chain_database_path(dir.path()).exists(),
            "failed migration must not leave chain.sqlite3; error was {error:?}"
        );
    }

    /// Online SQLite backup can be restored into a fresh data dir and matches tip.
    #[test]
    fn restore_from_online_backup_matches_tip_and_validates() {
        let source = tempfile::tempdir().expect("source");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        append_block(source.path(), &genesis).expect("genesis");
        append_block(source.path(), &child).expect("child");
        let original = load_blocks(source.path()).expect("load source");
        let original_tip = original.last().expect("tip").hash().expect("tip hash");

        let backup_path = source.path().join("backup-chain.sqlite3");
        backup_chain_database(source.path(), &backup_path).expect("backup");

        let restore = tempfile::tempdir().expect("restore");
        let dest_db = chain_database_path(restore.path());
        fs::create_dir_all(restore.path()).expect("mkdir");
        fs::copy(&backup_path, &dest_db).expect("copy backup into fresh data dir");

        verify_database_integrity(restore.path()).expect("restored integrity");
        let restored = load_blocks(restore.path()).expect("load restored");
        assert_eq!(restored.len(), original.len());
        assert_eq!(
            restored.last().expect("tip").hash().expect("restored tip"),
            original_tip
        );
        // Structural validation from genesis through tip.
        verify_chain_structure(&dest_db, &restored).expect("structure");
    }
}
