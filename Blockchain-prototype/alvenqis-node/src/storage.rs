use crate::error::{NodeError, NodeResult};
use alvenqis_core::{
    blake3_hash, cumulative_work, hash_to_hex, Block, Hash, Transaction as CoreTransaction,
};
use fs2::FileExt;
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, MAIN_DB,
};
use serde::Serialize;
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

const STORAGE_SCHEMA_VERSION: i64 = 2;
const STORAGE_APPLICATION_ID: i64 = 0x5649_5245; // "ALVE"
const MINIMUM_SAFE_SQLITE_VERSION: i32 = 3_051_003;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

type ValidatedBlockTokens = BTreeMap<i64, Vec<u8>>;
type ValidatedBlockTokenCache = BTreeMap<PathBuf, ValidatedBlockTokens>;

static VALIDATED_BLOCK_TOKEN_CACHE: OnceLock<Mutex<ValidatedBlockTokenCache>> = OnceLock::new();

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredTransaction {
    pub tx_hash: String,
    pub block_height: u64,
    pub transaction_position: usize,
    pub block_hash: String,
    pub block_base_fee_atomic: u64,
    pub transaction: CoreTransaction,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StorageIntegrityReport {
    pub schema_version: i64,
    pub canonical_block_count: u64,
    pub canonical_transaction_count: u64,
    pub genesis_hash: Option<String>,
    pub tip_hash: Option<String>,
    /// Diagnostic-only commitment over canonical block hashes. This is not a
    /// consensus field and never changes block validity.
    pub block_hash_merkle_root: Option<String>,
}

pub trait BlockStore {
    fn load_blocks(&self) -> NodeResult<Vec<Block>>;

    /// Validates a candidate against the caller's already-validated chain view, then
    /// commits it only if SQLite still has the same tip.
    fn append_validated<R, F>(
        &self,
        expected_tip: &str,
        candidate: &Block,
        validate: F,
    ) -> NodeResult<R>
    where
        F: FnOnce(&Block) -> NodeResult<R>;

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
        let database_path = chain_database_path(&self.data_dir);
        let mut connection = Connection::open_with_flags(
            &database_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_read_connection(&connection)?;
        let user_version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if user_version != STORAGE_SCHEMA_VERSION {
            drop(connection);
            self.migrate_database()?;
            connection = Connection::open_with_flags(
                &database_path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            configure_read_connection(&connection)?;
        }
        validate_schema(&connection, &database_path)?;
        Ok(connection)
    }

    fn migrate_database(&self) -> NodeResult<()> {
        let _lock = self.open_exclusive_lock()?;
        let database_path = chain_database_path(&self.data_dir);
        let mut connection = Connection::open_with_flags(
            chain_database_path(&self.data_dir),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_connection(&connection, true)?;
        migrate_schema_if_needed(&mut connection, &database_path)
    }

    fn open_write_connection(&self) -> NodeResult<Connection> {
        self.prepare_database(true)?;
        let database_path = chain_database_path(&self.data_dir);
        let mut connection = Connection::open_with_flags(
            &database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_connection(&connection, true)?;
        migrate_schema_if_needed(&mut connection, &database_path)?;
        Ok(connection)
    }

    pub fn load_tip_block(&self) -> NodeResult<Option<Block>> {
        let connection = self.open_read_connection()?;
        load_tip_block_from_connection(&connection, &chain_database_path(&self.data_dir))
    }

    pub fn load_block_at_height(&self, height: u64) -> NodeResult<Option<Block>> {
        let connection = self.open_read_connection()?;
        let mut blocks = load_blocks_range_from_connection(
            &connection,
            &chain_database_path(&self.data_dir),
            height,
            1,
        )?;
        Ok(blocks.pop())
    }

    pub fn load_blocks_range(&self, start_height: u64, limit: usize) -> NodeResult<Vec<Block>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let connection = self.open_read_connection()?;
        load_blocks_range_from_connection(
            &connection,
            &chain_database_path(&self.data_dir),
            start_height,
            limit,
        )
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

    pub fn load_transaction_by_hash(&self, tx_hash: &str) -> NodeResult<Option<StoredTransaction>> {
        let requested_hash = Hash::from_hex(tx_hash).map_err(NodeError::Input)?;
        let connection = self.open_read_connection()?;
        let database_path = chain_database_path(&self.data_dir);
        load_transaction_by_hash_from_connection(&connection, &database_path, &requested_hash)
    }

    pub fn existing_transaction_hashes(
        &self,
        tx_hashes: &[String],
    ) -> NodeResult<BTreeSet<String>> {
        let connection = self.open_read_connection()?;
        let database_path = chain_database_path(&self.data_dir);
        let mut existing = BTreeSet::new();
        for tx_hash in tx_hashes {
            let Ok(requested_hash) = Hash::from_hex(tx_hash) else {
                continue;
            };
            if load_transaction_by_hash_from_connection(
                &connection,
                &database_path,
                &requested_hash,
            )?
            .is_some()
            {
                existing.insert(tx_hash.clone());
            }
        }
        Ok(existing)
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

fn load_transaction_by_hash_from_connection(
    connection: &Connection,
    database_path: &Path,
    requested_hash: &Hash,
) -> NodeResult<Option<StoredTransaction>> {
    let location: Option<(i64, i64)> = connection
        .query_row(
            "SELECT block_height, tx_position
             FROM canonical_transactions
             WHERE tx_hash = ?1
             ORDER BY block_height ASC, tx_position ASC
             LIMIT 1",
            params![requested_hash.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((block_height, transaction_position)) = location else {
        return Ok(None);
    };
    let block_height = u64::try_from(block_height).map_err(|_| {
        invalid_database(
            database_path,
            format!("negative transaction index height {block_height}"),
        )
    })?;
    let transaction_position = usize::try_from(transaction_position).map_err(|_| {
        invalid_database(
            database_path,
            format!("negative transaction index position {transaction_position}"),
        )
    })?;
    let block = load_blocks_range_from_connection(connection, database_path, block_height, 1)?
        .pop()
        .ok_or_else(|| {
            invalid_database(
                database_path,
                format!("transaction index references missing block height {block_height}"),
            )
        })?;
    let transaction = block
        .transactions
        .get(transaction_position)
        .cloned()
        .ok_or_else(|| {
            invalid_database(
                database_path,
                format!(
                    "transaction index position {transaction_position} is outside block height {block_height}"
                ),
            )
        })?;
    let actual_hash = transaction.tx_hash();
    if actual_hash != *requested_hash {
        return Err(invalid_database(
            database_path,
            format!(
                "transaction index hash mismatch at height {block_height} position {transaction_position}"
            ),
        ));
    }
    Ok(Some(StoredTransaction {
        tx_hash: hash_to_hex(&actual_hash),
        block_height,
        transaction_position,
        block_hash: hash_to_hex(&block.hash()?),
        block_base_fee_atomic: block.header.base_fee_atomic,
        transaction,
    }))
}

impl BlockStore for SqliteBlockStore {
    fn load_blocks(&self) -> NodeResult<Vec<Block>> {
        let connection = self.open_read_connection()?;
        load_blocks_from_connection(&connection, &chain_database_path(&self.data_dir), false)
    }

    fn append_validated<R, F>(
        &self,
        expected_tip: &str,
        candidate: &Block,
        validate: F,
    ) -> NodeResult<R>
    where
        F: FnOnce(&Block) -> NodeResult<R>,
    {
        let result = validate(candidate)?;
        let mut connection = self.open_write_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let tip =
            load_tip_block_from_connection(&transaction, &chain_database_path(&self.data_dir))?;
        let actual_tip = match tip.as_ref() {
            Some(block) => hash_to_hex(&block.hash()?),
            None => "none".to_owned(),
        };
        if actual_tip != expected_tip {
            return Err(NodeError::StaleChainTip {
                expected: expected_tip.to_owned(),
                actual: actual_tip,
            });
        }
        verify_tip_extension(tip.as_ref(), candidate)?;
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
    index_block_transactions(connection, block)?;
    Ok(())
}

fn index_block_transactions(connection: &Connection, block: &Block) -> NodeResult<()> {
    let block_height = height_to_i64(block.header.height)?;
    for (position, transaction) in block.transactions.iter().enumerate() {
        let position = i64::try_from(position).map_err(|_| {
            NodeError::Input(format!(
                "transaction position {position} exceeds SQLite range"
            ))
        })?;
        let tx_hash = transaction.tx_hash();
        connection.execute(
            "INSERT INTO canonical_transactions
             (block_height, tx_position, tx_hash)
             VALUES (?1, ?2, ?3)",
            params![block_height, position, tx_hash.as_bytes().as_slice()],
        )?;
    }
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
    validation_token: Vec<u8>,
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
    let validation_token = block_validation_token(&hash, &block_json);
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
    let recomputed_merkle_root = block.recompute_merkle_root().map_err(|error| {
        invalid_database(
            database_path,
            format!("cannot recompute transaction Merkle root at height {height}: {error}"),
        )
    })?;
    if recomputed_merkle_root != block.header.merkle_root {
        return Err(invalid_database(
            database_path,
            format!("transaction Merkle root mismatch at height {height}"),
        ));
    }
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
        validation_token,
        block,
    })
}

fn block_validation_token(stored_hash: &[u8], block_json: &[u8]) -> Vec<u8> {
    let body_hash = blake3_hash(block_json);
    let mut token = Vec::with_capacity(stored_hash.len() + body_hash.as_bytes().len());
    token.extend_from_slice(stored_hash);
    token.extend_from_slice(body_hash.as_bytes());
    token
}

fn load_validated_canonical_rows_from_height(
    connection: &Connection,
    database_path: &Path,
    start_height: i64,
    cached_tokens: &BTreeMap<i64, Vec<u8>>,
    row_limit: Option<usize>,
) -> NodeResult<Vec<ValidatedCanonicalBlockRow>> {
    let mut statement = connection.prepare(if row_limit.is_some() {
        "SELECT height, hash, previous_hash, network_id, block_json
         FROM canonical_blocks
         WHERE height >= ?1
         ORDER BY height ASC
         LIMIT ?2"
    } else {
        "SELECT height, hash, previous_hash, network_id, block_json
         FROM canonical_blocks
         WHERE height >= ?1
         ORDER BY height ASC"
    })?;
    let mut rows = match row_limit {
        Some(limit) => {
            let limit = i64::try_from(limit).map_err(|_| {
                NodeError::Input("block range limit exceeds SQLite bounds".to_owned())
            })?;
            statement.query(params![start_height, limit])?
        }
        None => statement.query(params![start_height])?,
    };
    let mut previous_stored_hash: Option<Vec<u8>> = None;
    let mut expected_height = start_height;
    let mut validated_rows = Vec::new();
    while let Some(row) = rows.next()? {
        let stored = stored_canonical_block_row(row)?;
        let hash_was_validated = cached_tokens.get(&stored.height).is_some_and(|cached| {
            cached == &block_validation_token(&stored.hash, &stored.block_json)
        });
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
        None,
    )?;
    Ok(rows
        .into_iter()
        .filter(|row| row.height >= start_height)
        .map(|row| row.block)
        .collect())
}

fn load_blocks_range_from_connection(
    connection: &Connection,
    database_path: &Path,
    start_height: u64,
    limit: usize,
) -> NodeResult<Vec<Block>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let start_height = height_to_i64(start_height)?;
    let include_predecessor = start_height > 0;
    let query_start_height = if include_predecessor {
        start_height - 1
    } else {
        0
    };
    let row_limit = limit
        .checked_add(usize::from(include_predecessor))
        .ok_or_else(|| NodeError::Input("block range limit overflow".to_owned()))?;
    let rows = load_validated_canonical_rows_from_height(
        connection,
        database_path,
        query_start_height,
        &BTreeMap::new(),
        Some(row_limit),
    )?;
    Ok(rows
        .into_iter()
        .filter(|row| row.height >= start_height)
        .take(limit)
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
    let validation_cache = VALIDATED_BLOCK_TOKEN_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let cached_tokens = validation_cache
        .lock()
        .map_err(|_| NodeError::Input("block validation cache lock poisoned".to_owned()))?
        .get(database_path)
        .cloned()
        .unwrap_or_default();
    let rows = load_validated_canonical_rows_from_height(
        connection,
        database_path,
        0,
        &cached_tokens,
        None,
    )?;
    let mut validated_tokens = BTreeMap::new();
    let mut blocks = Vec::with_capacity(rows.len());
    for row in rows {
        validated_tokens.insert(row.height, row.validation_token);
        blocks.push(row.block);
    }

    if blocks.is_empty() && !allow_empty {
        return Err(NodeError::ChainNotInitialized(database_path.to_path_buf()));
    }
    validation_cache
        .lock()
        .map_err(|_| NodeError::Input("block validation cache lock poisoned".to_owned()))?
        .insert(database_path.to_path_buf(), validated_tokens);
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
         CREATE TABLE canonical_transactions (
           block_height INTEGER NOT NULL CHECK(block_height >= 0),
           tx_position INTEGER NOT NULL CHECK(tx_position >= 0),
           tx_hash BLOB NOT NULL CHECK(length(tx_hash) = 32),
           PRIMARY KEY(block_height, tx_position),
           FOREIGN KEY(block_height) REFERENCES canonical_blocks(height) ON DELETE CASCADE
         ) STRICT;
         CREATE INDEX canonical_transactions_hash_idx
           ON canonical_transactions(tx_hash);
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
         PRAGMA user_version = 2;
         INSERT INTO storage_metadata(key, value) VALUES ('schema_version', '2');
         INSERT INTO storage_metadata(key, value) VALUES ('backend', 'sqlite');
         COMMIT;",
    )?;
    validate_schema(connection, Path::new(CHAIN_DATABASE_FILE_NAME))
}

fn migrate_schema_if_needed(connection: &mut Connection, database_path: &Path) -> NodeResult<()> {
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
    if application_id == STORAGE_APPLICATION_ID
        && user_version == STORAGE_SCHEMA_VERSION
        && metadata_version.as_deref() == Some("2")
    {
        return Ok(());
    }
    if application_id != STORAGE_APPLICATION_ID
        || user_version != 1
        || metadata_version.as_deref() != Some("1")
    {
        return validate_schema(connection, database_path);
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE canonical_transactions (
           block_height INTEGER NOT NULL CHECK(block_height >= 0),
           tx_position INTEGER NOT NULL CHECK(tx_position >= 0),
           tx_hash BLOB NOT NULL CHECK(length(tx_hash) = 32),
           PRIMARY KEY(block_height, tx_position),
           FOREIGN KEY(block_height) REFERENCES canonical_blocks(height) ON DELETE CASCADE
         ) STRICT;
         CREATE INDEX canonical_transactions_hash_idx
           ON canonical_transactions(tx_hash);",
    )?;
    backfill_transaction_index(&transaction, database_path)?;
    set_metadata(&transaction, "schema_version", "2")?;
    transaction.pragma_update(None, "user_version", STORAGE_SCHEMA_VERSION)?;
    transaction.commit()?;
    validate_schema(connection, database_path)
}

fn backfill_transaction_index(connection: &Connection, database_path: &Path) -> NodeResult<()> {
    let mut statement = connection
        .prepare("SELECT height, block_json FROM canonical_blocks ORDER BY height ASC")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    for row in rows {
        let (stored_height, block_json) = row?;
        let block: Block = serde_json::from_slice(&block_json).map_err(|error| {
            invalid_database(
                database_path,
                format!(
                    "cannot decode block at height {stored_height} while migrating transaction index: {error}"
                ),
            )
        })?;
        if height_to_i64(block.header.height)? != stored_height {
            return Err(invalid_database(
                database_path,
                format!(
                    "stored block height mismatch while migrating transaction index: row={stored_height}, block={}",
                    block.header.height
                ),
            ));
        }
        index_block_transactions(connection, &block)?;
    }
    Ok(())
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
        || metadata_version.as_deref() != Some("2")
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

pub fn load_transaction_by_hash(
    data_dir: &Path,
    tx_hash: &str,
) -> NodeResult<Option<StoredTransaction>> {
    SqliteBlockStore::new(data_dir).load_transaction_by_hash(tx_hash)
}

pub fn existing_transaction_hashes(
    data_dir: &Path,
    tx_hashes: &[String],
) -> NodeResult<BTreeSet<String>> {
    SqliteBlockStore::new(data_dir).existing_transaction_hashes(tx_hashes)
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

pub fn verify_database_integrity(data_dir: &Path) -> NodeResult<StorageIntegrityReport> {
    let store = SqliteBlockStore::new(data_dir);
    let connection = store.open_read_connection()?;
    verify_database_integrity_connection(&connection, &chain_database_path(data_dir))
}

fn verify_database_integrity_connection(
    connection: &Connection,
    database_path: &Path,
) -> NodeResult<StorageIntegrityReport> {
    let mut integrity_statement = connection.prepare("PRAGMA integrity_check")?;
    let integrity_rows = integrity_statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut integrity_results = Vec::new();
    for result in integrity_rows {
        integrity_results.push(result?);
        if integrity_results.len() >= 20 {
            break;
        }
    }
    if integrity_results.as_slice() != ["ok"] {
        return Err(invalid_database(
            database_path,
            format!(
                "SQLite integrity_check failed: {}",
                integrity_results.join("; ")
            ),
        ));
    }

    let mut foreign_key_statement = connection.prepare("PRAGMA foreign_key_check")?;
    let mut foreign_key_rows = foreign_key_statement.query([])?;
    if let Some(row) = foreign_key_rows.next()? {
        let table: String = row.get(0)?;
        let row_id: Option<i64> = row.get(1)?;
        let parent: String = row.get(2)?;
        return Err(invalid_database(
            database_path,
            format!(
                "SQLite foreign_key_check failed: table={table} row_id={row_id:?} parent={parent}"
            ),
        ));
    }

    let canonical_rows = load_validated_canonical_rows_from_height(
        connection,
        database_path,
        0,
        &BTreeMap::new(),
        None,
    )?;
    let mut block_hashes = Vec::with_capacity(canonical_rows.len());
    let mut genesis_hash = None;
    let mut tip_hash = None;
    for row in &canonical_rows {
        let block_hash_bytes: [u8; 32] = row.hash.as_slice().try_into().map_err(|_| {
            invalid_database(
                database_path,
                format!("invalid canonical block hash at height {}", row.height),
            )
        })?;
        let block_hash = Hash::from_bytes(block_hash_bytes);
        let block_hash_hex = hash_to_hex(&block_hash);
        genesis_hash.get_or_insert_with(|| block_hash_hex.clone());
        tip_hash = Some(block_hash_hex);
        block_hashes.push(block_hash);
    }

    let canonical_transaction_count =
        verify_transaction_index_connection(connection, database_path)?;
    let schema_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    Ok(StorageIntegrityReport {
        schema_version,
        canonical_block_count: u64::try_from(canonical_rows.len()).map_err(|_| {
            invalid_database(
                database_path,
                "canonical block count exceeds report range".to_owned(),
            )
        })?,
        canonical_transaction_count,
        genesis_hash,
        tip_hash,
        block_hash_merkle_root: diagnostic_block_hash_merkle_root(&block_hashes)
            .map(|root| hash_to_hex(&root)),
    })
}

fn verify_transaction_index_connection(
    connection: &Connection,
    database_path: &Path,
) -> NodeResult<u64> {
    let actual_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM canonical_transactions", [], |row| {
            row.get(0)
        })?;
    let mut expected_count = 0_i64;
    let mut statement = connection
        .prepare("SELECT height, block_json FROM canonical_blocks ORDER BY height ASC")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    for row in rows {
        let (height, block_json) = row?;
        let block: Block = serde_json::from_slice(&block_json).map_err(|error| {
            invalid_database(
                database_path,
                format!("cannot decode block at height {height} while verifying index: {error}"),
            )
        })?;
        for (position, transaction) in block.transactions.iter().enumerate() {
            let position = i64::try_from(position).map_err(|_| {
                invalid_database(
                    database_path,
                    format!("transaction position {position} exceeds SQLite range"),
                )
            })?;
            let indexed_hash: Option<Vec<u8>> = connection
                .query_row(
                    "SELECT tx_hash FROM canonical_transactions
                     WHERE block_height = ?1 AND tx_position = ?2",
                    params![height, position],
                    |row| row.get(0),
                )
                .optional()?;
            if indexed_hash.as_deref() != Some(transaction.tx_hash().as_bytes().as_slice()) {
                return Err(invalid_database(
                    database_path,
                    format!("transaction index mismatch at height {height} position {position}"),
                ));
            }
            expected_count = expected_count.checked_add(1).ok_or_else(|| {
                invalid_database(database_path, "transaction index count overflow".to_owned())
            })?;
        }
    }
    if actual_count != expected_count {
        return Err(invalid_database(
            database_path,
            format!(
                "transaction index row count mismatch: expected {expected_count}, found {actual_count}"
            ),
        ));
    }
    u64::try_from(expected_count).map_err(|_| {
        invalid_database(
            database_path,
            format!("invalid canonical transaction count {expected_count}"),
        )
    })
}

fn diagnostic_block_hash_merkle_root(block_hashes: &[Hash]) -> Option<Hash> {
    if block_hashes.is_empty() {
        return None;
    }

    let mut level = block_hashes
        .iter()
        .enumerate()
        .map(|(height, block_hash)| {
            let mut leaf = Vec::with_capacity(45);
            leaf.extend_from_slice(b"alve-storage-leaf-v1");
            leaf.extend_from_slice(&(height as u64).to_le_bytes());
            leaf.extend_from_slice(block_hash.as_bytes());
            blake3_hash(&leaf)
        })
        .collect::<Vec<_>>();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().expect("non-empty Merkle level"));
        }
        level = level
            .chunks_exact(2)
            .map(|pair| {
                let mut branch = Vec::with_capacity(86);
                branch.extend_from_slice(b"alve-storage-branch-v1");
                branch.extend_from_slice(pair[0].as_bytes());
                branch.extend_from_slice(pair[1].as_bytes());
                blake3_hash(&branch)
            })
            .collect();
    }
    level.pop()
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
    use alvenqis_core::{
        devnet_child_block_with_difficulty, devnet_genesis, hash_to_hex, Address, PrivateKey,
        BLOCK_TIME_SECONDS,
    };

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
    fn cached_load_rejects_tampered_block_body() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");
        store.load_blocks().expect("warm validation cache");

        let connection = store.open_write_connection().expect("open");
        let block_json: Vec<u8> = connection
            .query_row(
                "SELECT block_json FROM canonical_blocks WHERE height = 0",
                [],
                |row| row.get(0),
            )
            .expect("stored block");
        let mut tampered: Block = serde_json::from_slice(&block_json).expect("decode block");
        tampered.header.timestamp = tampered.header.timestamp.saturating_add(1);
        connection
            .execute(
                "UPDATE canonical_blocks SET block_json = ?1 WHERE height = 0",
                params![serde_json::to_vec(&tampered).expect("encode tampered block")],
            )
            .expect("tamper block body");
        drop(connection);

        let error = store
            .load_blocks()
            .expect_err("cached validation must not hide block-body corruption");
        assert!(matches!(error, NodeError::InvalidChainDatabase { .. }));
    }

    #[test]
    fn integrity_check_rejects_merkle_root_mismatch_when_stored_hash_matches() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");

        let mut tampered = genesis;
        tampered.header.merkle_root = Hash::zero();
        let tampered_hash = tampered.hash().expect("tampered block hash");
        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_blocks
                 SET hash = ?1, block_json = ?2
                 WHERE height = 0",
                params![
                    tampered_hash.as_bytes().as_slice(),
                    serde_json::to_vec(&tampered).expect("encode tampered block")
                ],
            )
            .expect("tamper merkle commitment");
        drop(connection);

        let error = verify_database_integrity(dir.path())
            .expect_err("integrity check must recompute transaction Merkle roots");
        assert!(matches!(error, NodeError::InvalidChainDatabase { .. }));
    }

    #[test]
    fn integrity_report_commits_canonical_block_order() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        store.append_with_tip_link(&genesis).expect("genesis");
        let genesis_report = verify_database_integrity(dir.path()).expect("genesis report");

        let child = linked_child(&genesis);
        store.append_with_tip_link(&child).expect("child");
        let child_report = verify_database_integrity(dir.path()).expect("child report");

        assert_eq!(genesis_report.canonical_block_count, 1);
        assert_eq!(genesis_report.canonical_transaction_count, 1);
        assert_eq!(child_report.canonical_block_count, 2);
        assert_eq!(child_report.canonical_transaction_count, 2);
        assert_eq!(
            child_report.tip_hash,
            Some(hash_to_hex(&child.hash().expect("child hash")))
        );
        assert_ne!(
            genesis_report.block_hash_merkle_root,
            child_report.block_hash_merkle_root
        );
    }

    #[test]
    fn transaction_index_follows_append_and_reorg() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address(),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
            4,
        )
        .expect("child");
        let child_transaction = child.transactions[0].clone();
        let child_transaction_hash = hash_to_hex(&child_transaction.tx_hash());
        let child_block_hash = hash_to_hex(&child.hash().expect("child hash"));
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");

        let indexed = store
            .load_transaction_by_hash(&child_transaction_hash)
            .expect("lookup")
            .expect("indexed transaction");
        assert_eq!(indexed.block_height, 1);
        assert_eq!(indexed.transaction_position, 0);
        assert_eq!(indexed.block_hash, child_block_hash);
        assert_eq!(indexed.transaction, child_transaction);
        let existing = store
            .existing_transaction_hashes(&[
                child_transaction_hash.clone(),
                hash_to_hex(&Hash::zero()),
                "malformed".to_owned(),
            ])
            .expect("batch lookup");
        assert_eq!(existing, BTreeSet::from([child_transaction_hash.clone()]));

        store
            .replace_validated(
                &child_block_hash,
                std::slice::from_ref(&genesis),
                |_current, _candidate| Ok(()),
            )
            .expect("replace");
        assert!(store
            .load_transaction_by_hash(&child_transaction_hash)
            .expect("lookup after reorg")
            .is_none());
        assert!(store
            .existing_transaction_hashes(&[child_transaction_hash])
            .expect("batch lookup after reorg")
            .is_empty());
    }

    #[test]
    fn schema_v1_migration_backfills_transaction_index() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let transaction_hash = hash_to_hex(&genesis.transactions[0].tx_hash());
        store.append_with_tip_link(&genesis).expect("genesis");

        let connection = store.open_write_connection().expect("open");
        connection
            .execute_batch(
                "DROP TABLE canonical_transactions;
                 PRAGMA user_version = 1;
                 UPDATE storage_metadata SET value = '1' WHERE key = 'schema_version';",
            )
            .expect("downgrade schema fixture");
        drop(connection);

        let indexed = store
            .load_transaction_by_hash(&transaction_hash)
            .expect("migrate and lookup")
            .expect("backfilled transaction");
        assert_eq!(indexed.block_height, 0);
        assert_eq!(indexed.transaction_position, 0);
        let connection = store.open_read_connection().expect("open migrated");
        let schema_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        assert_eq!(schema_version, STORAGE_SCHEMA_VERSION);
    }

    #[test]
    fn transaction_lookup_rejects_inconsistent_index_position() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let transaction_hash = genesis.transactions[0].tx_hash();
        store.append_with_tip_link(&genesis).expect("genesis");

        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_transactions SET tx_position = 99 WHERE tx_hash = ?1",
                params![transaction_hash.as_bytes().as_slice()],
            )
            .expect("tamper index");
        drop(connection);

        let error = store
            .load_transaction_by_hash(&hash_to_hex(&transaction_hash))
            .expect_err("inconsistent index must fail");
        assert!(matches!(error, NodeError::InvalidChainDatabase { .. }));
        let integrity_error =
            verify_database_integrity(dir.path()).expect_err("integrity check must fail");
        assert!(matches!(
            integrity_error,
            NodeError::InvalidChainDatabase { .. }
        ));
    }

    #[test]
    fn transaction_index_lookup_does_not_load_unrelated_suffix() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = devnet_child_block_with_difficulty(
            &genesis,
            &miner_address(),
            genesis.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
            4,
        )
        .expect("child");
        let grandchild = devnet_child_block_with_difficulty(
            &child,
            &miner_address(),
            child.header.timestamp + BLOCK_TIME_SECONDS,
            vec![],
            4,
        )
        .expect("grandchild");
        let child_transaction_hash = hash_to_hex(&child.transactions[0].tx_hash());
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");
        store.append_with_tip_link(&grandchild).expect("grandchild");

        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_blocks
                 SET network_id = 'tampered-beyond-indexed-block'
                 WHERE height = 2",
                [],
            )
            .expect("tamper unrelated suffix");
        drop(connection);

        let indexed = store
            .load_transaction_by_hash(&child_transaction_hash)
            .expect("bounded transaction lookup")
            .expect("indexed transaction");
        assert_eq!(indexed.block_height, 1);
        assert_eq!(indexed.transaction, child.transactions[0]);
        let full_chain_error = store
            .load_blocks()
            .expect_err("full chain load must still detect unrelated corruption");
        assert!(matches!(
            full_chain_error,
            NodeError::InvalidChainDatabase { .. }
        ));
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
    fn bounded_range_and_point_queries_do_not_load_the_full_suffix() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        let grandchild = linked_child(&child);
        store.append_with_tip_link(&genesis).expect("genesis");
        store.append_with_tip_link(&child).expect("child");
        store.append_with_tip_link(&grandchild).expect("grandchild");

        assert_eq!(
            store.load_block_at_height(1).expect("point"),
            Some(child.clone())
        );
        assert_eq!(
            store.load_blocks_range(1, 1).expect("bounded range"),
            vec![child.clone()]
        );
        assert!(store
            .load_blocks_range(1, 0)
            .expect("zero-length range")
            .is_empty());
        assert_eq!(
            store.load_blocks_range(2, 8).expect("short tail"),
            vec![grandchild]
        );
        assert_eq!(store.load_block_at_height(3).expect("missing"), None);

        let connection = store.open_write_connection().expect("open");
        connection
            .execute(
                "UPDATE canonical_blocks
                 SET network_id = 'tampered-beyond-range'
                 WHERE height = 2",
                [],
            )
            .expect("tamper beyond bounded range");
        assert_eq!(
            store
                .load_blocks_range(1, 1)
                .expect("bounded query must stop before tampered row"),
            vec![child]
        );
        let error = store
            .load_blocks_from_height(1)
            .expect_err("unbounded suffix must still validate every row");
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
    fn validated_tip_append_rejects_concurrent_tip_change_without_mutation() {
        let dir = tempfile::tempdir().expect("temp");
        let store = SqliteBlockStore::new(dir.path());
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let child = linked_child(&genesis);
        let grandchild = linked_child(&child);
        store.append_with_tip_link(&genesis).expect("genesis");
        let expected_tip = hash_to_hex(&genesis.hash().expect("genesis hash"));
        store
            .append_with_tip_link(&child)
            .expect("concurrent child");

        let error = store
            .append_validated(&expected_tip, &grandchild, |_| Ok(()))
            .expect_err("changed tip must fail");

        assert!(matches!(error, NodeError::StaleChainTip { .. }));
        assert_eq!(store.canonical_block_count().expect("count"), 2);
        assert_eq!(store.load_tip_block().expect("tip"), Some(child));
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
