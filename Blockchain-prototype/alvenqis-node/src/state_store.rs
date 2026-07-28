use crate::error::{NodeError, NodeResult};
use crate::mempool::{tx_hash_string, PendingTransactionRecord};
use alvenqis_core::{block_work, hash_to_hex, sha256, Block, Chain, LedgerState, Network};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use fs2::FileExt;
use rand::{rngs::OsRng, RngCore};
use rocksdb::{
    backup::{BackupEngine, BackupEngineOptions, RestoreOptions},
    BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, DBCompressionType, DBRecoveryMode, Env,
    IteratorMode, Options, ReadOptions, WriteBatch, WriteOptions, DB,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

pub const STATE_DATABASE_DIR_NAME: &str = "state.rocksdb";
pub const STATE_DATABASE_LOCK_FILE_NAME: &str = "state.rocksdb.lock";
pub const STORAGE_KEY_FILE_ENV: &str = "ALVENQIS_STORAGE_KEY_FILE";
pub const REQUIRE_STORAGE_ENCRYPTION_ENV: &str = "ALVENQIS_REQUIRE_STORAGE_ENCRYPTION";
pub const ALLOW_PLAINTEXT_MIGRATION_ENV: &str = "ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION";

const DEFAULT_STORAGE_KEY_FILE: &str = "/run/secrets/alvenqis_storage_key";
const ROCKS_SCHEMA_VERSION: u32 = 1;
const ENCRYPTED_VALUE_MAGIC: &[u8; 8] = b"ALVENC01";
const PLAINTEXT_VALUE_MAGIC: &[u8; 8] = b"ALVCLR01";
const XCHACHA_NONCE_BYTES: usize = 24;
const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const ENCRYPTION_MIGRATION_BATCH_SIZE: usize = 1_024;
const RESTORE_STAGE_PREFIX: &str = ".alvenqis-state-restore-";
const META_SCHEMA_VERSION: &[u8] = b"schema_version";
const META_BACKEND: &[u8] = b"backend";
const META_ENCRYPTION: &[u8] = b"encryption";
const META_KEY_ID: &[u8] = b"key_id";
const META_MEMPOOL_INITIALIZED: &[u8] = b"mempool_initialized";
const STATE_SNAPSHOT_KEY: &[u8] = b"canonical";

const CF_METADATA: &str = "metadata";
const CF_CANONICAL_STATE: &str = "canonical_state";
const CF_ACCOUNTS: &str = "accounts";
const CF_BLOCK_METADATA: &str = "block_metadata";
const CF_RECENT_TRANSACTIONS: &str = "recent_transactions";
const CF_MEMPOOL: &str = "mempool";
const COLUMN_FAMILIES: [&str; 6] = [
    CF_METADATA,
    CF_CANONICAL_STATE,
    CF_ACCOUNTS,
    CF_BLOCK_METADATA,
    CF_RECENT_TRANSACTIONS,
    CF_MEMPOOL,
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedChainState {
    pub schema_version: u32,
    pub network_id: String,
    pub block_count: u64,
    pub tip_height: u64,
    pub tip_hash: String,
    pub tip_timestamp: u64,
    pub state: LedgerState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedAccountState {
    balance_atomic: u64,
    next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedBlockMetadata {
    height: u64,
    hash: String,
    previous_hash: String,
    network_id: String,
    timestamp: u64,
    difficulty_leading_zero_bits: u8,
    transaction_count: usize,
    cumulative_work: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RocksStateStatus {
    pub schema_version: u32,
    pub network_id: String,
    pub block_count: u64,
    pub tip_height: u64,
    pub tip_hash: String,
    pub accounts: usize,
    pub recent_transactions: usize,
    pub mempool_transactions: usize,
    pub encryption: String,
    pub key_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RocksBackupInfo {
    pub backup_id: u32,
    pub timestamp: i64,
    pub size: u64,
    pub files: u32,
    pub network_id: String,
    pub block_count: u64,
    pub tip_height: u64,
    pub tip_hash: String,
    pub encryption: String,
    pub key_id: String,
}

#[derive(Debug)]
enum ValueCipher {
    Plaintext {
        allow_plaintext_migration: bool,
    },
    XChaCha20Poly1305 {
        key: Zeroizing<[u8; 32]>,
        key_id: String,
        allow_plaintext_migration: bool,
    },
}

impl ValueCipher {
    fn from_environment() -> NodeResult<Self> {
        let required = environment_flag(REQUIRE_STORAGE_ENCRYPTION_ENV)?;
        let allow_plaintext_migration = environment_flag(ALLOW_PLAINTEXT_MIGRATION_ENV)?;
        let configured_path = std::env::var_os(STORAGE_KEY_FILE_ENV).map(PathBuf::from);
        let default_path = PathBuf::from(DEFAULT_STORAGE_KEY_FILE);
        let key_path = configured_path.or_else(|| default_path.exists().then_some(default_path));

        match key_path {
            Some(path) => {
                let key = read_storage_key(&path)?;
                let key_id = hash_to_hex(&sha256(key.as_ref()))[..16].to_owned();
                Ok(Self::XChaCha20Poly1305 {
                    key,
                    key_id,
                    allow_plaintext_migration,
                })
            }
            None if required => Err(NodeError::Input(format!(
                "{REQUIRE_STORAGE_ENCRYPTION_ENV}=true but no storage key is available; set {STORAGE_KEY_FILE_ENV}"
            ))),
            None => Ok(Self::Plaintext {
                allow_plaintext_migration,
            }),
        }
    }

    #[cfg(test)]
    fn encrypted_for_test(key: [u8; 32]) -> Self {
        let key_id = hash_to_hex(&sha256(&key))[..16].to_owned();
        Self::XChaCha20Poly1305 {
            key: Zeroizing::new(key),
            key_id,
            allow_plaintext_migration: false,
        }
    }

    fn mode(&self) -> &'static str {
        match self {
            Self::Plaintext { .. } => "plaintext",
            Self::XChaCha20Poly1305 { .. } => "xchacha20poly1305",
        }
    }

    fn key_id(&self) -> &str {
        match self {
            Self::Plaintext { .. } => "none",
            Self::XChaCha20Poly1305 { key_id, .. } => key_id,
        }
    }

    fn allow_plaintext_migration(&self) -> bool {
        match self {
            Self::Plaintext {
                allow_plaintext_migration,
            }
            | Self::XChaCha20Poly1305 {
                allow_plaintext_migration,
                ..
            } => *allow_plaintext_migration,
        }
    }

    fn disable_plaintext_migration(&mut self) {
        match self {
            Self::Plaintext {
                allow_plaintext_migration,
            }
            | Self::XChaCha20Poly1305 {
                allow_plaintext_migration,
                ..
            } => *allow_plaintext_migration = false,
        }
    }

    fn seal(&self, column_family: &str, key: &[u8], value: &[u8]) -> NodeResult<Vec<u8>> {
        match self {
            Self::Plaintext { .. } => {
                let mut encoded = Vec::with_capacity(PLAINTEXT_VALUE_MAGIC.len() + value.len());
                encoded.extend_from_slice(PLAINTEXT_VALUE_MAGIC);
                encoded.extend_from_slice(value);
                Ok(encoded)
            }
            Self::XChaCha20Poly1305 {
                key: storage_key, ..
            } => {
                let cipher = XChaCha20Poly1305::new_from_slice(storage_key.as_ref())
                    .map_err(|_| NodeError::Input("invalid storage encryption key".to_owned()))?;
                let mut nonce_bytes = [0_u8; XCHACHA_NONCE_BYTES];
                OsRng.try_fill_bytes(&mut nonce_bytes).map_err(|error| {
                    NodeError::Input(format!(
                        "operating-system randomness failed while sealing RocksDB value: {error}"
                    ))
                })?;
                let nonce = XNonce::from_slice(&nonce_bytes);
                let aad = value_aad(column_family, key);
                let ciphertext = cipher
                    .encrypt(
                        nonce,
                        Payload {
                            msg: value,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| NodeError::Input("storage value encryption failed".to_owned()))?;
                let mut encoded = Vec::with_capacity(
                    ENCRYPTED_VALUE_MAGIC.len() + XCHACHA_NONCE_BYTES + ciphertext.len(),
                );
                encoded.extend_from_slice(ENCRYPTED_VALUE_MAGIC);
                encoded.extend_from_slice(&nonce_bytes);
                encoded.extend_from_slice(&ciphertext);
                Ok(encoded)
            }
        }
    }

    fn open(&self, column_family: &str, key: &[u8], value: &[u8]) -> NodeResult<Vec<u8>> {
        if let Some(plaintext) = value.strip_prefix(PLAINTEXT_VALUE_MAGIC) {
            if matches!(self, Self::XChaCha20Poly1305 { .. })
                && !self.allow_plaintext_migration()
            {
                return Err(NodeError::Input(format!(
                    "plaintext RocksDB value rejected in encrypted mode for column family {column_family}"
                )));
            }
            return Ok(plaintext.to_vec());
        }

        let encrypted = value
            .strip_prefix(ENCRYPTED_VALUE_MAGIC)
            .ok_or_else(|| {
                NodeError::Input(format!(
                    "unknown RocksDB value envelope in column family {column_family}"
                ))
            })?;
        let (nonce_bytes, ciphertext) = encrypted
            .split_at_checked(XCHACHA_NONCE_BYTES)
            .ok_or_else(|| NodeError::Input("truncated encrypted RocksDB value".to_owned()))?;
        let Self::XChaCha20Poly1305 {
            key: storage_key, ..
        } = self
        else {
            return Err(NodeError::Input(format!(
                "encrypted RocksDB value requires {STORAGE_KEY_FILE_ENV}"
            )));
        };
        let cipher = XChaCha20Poly1305::new_from_slice(storage_key.as_ref())
            .map_err(|_| NodeError::Input("invalid storage encryption key".to_owned()))?;
        let nonce = XNonce::from_slice(nonce_bytes);
        let aad = value_aad(column_family, key);
        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| {
                NodeError::Input(format!(
                    "RocksDB value authentication failed in column family {column_family}"
                ))
            })
    }
}

struct RocksStateStore {
    database_path: PathBuf,
    database: DB,
    cipher: ValueCipher,
    _lock: File,
}

impl RocksStateStore {
    fn open(data_dir: &Path) -> NodeResult<Self> {
        Self::open_with_cipher(data_dir, ValueCipher::from_environment()?)
    }

    fn open_existing(data_dir: &Path) -> NodeResult<Self> {
        Self::open_existing_with_cipher(data_dir, ValueCipher::from_environment()?)
    }

    fn open_with_cipher(data_dir: &Path, cipher: ValueCipher) -> NodeResult<Self> {
        Self::open_internal(data_dir, cipher, true)
    }

    fn open_existing_with_cipher(data_dir: &Path, cipher: ValueCipher) -> NodeResult<Self> {
        Self::open_internal(data_dir, cipher, false)
    }

    fn open_internal(
        data_dir: &Path,
        cipher: ValueCipher,
        create_if_missing: bool,
    ) -> NodeResult<Self> {
        ensure_directory(data_dir, create_if_missing, "RocksDB data directory")?;
        let database_path = state_database_path(data_dir);
        validate_database_location(&database_path, create_if_missing)?;
        let lock = acquire_exclusive_lock(
            &data_dir.join(STATE_DATABASE_LOCK_FILE_NAME),
            "RocksDB state database",
        )?;
        let database_existed = validate_database_location(&database_path, create_if_missing)?;
        if database_existed {
            reject_unknown_column_families(&database_path)?;
        }
        let options = database_options(create_if_missing);
        let database = DB::open_cf_descriptors(
            &options,
            &database_path,
            COLUMN_FAMILIES
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(*name, column_family_options()))
                .collect::<Vec<_>>(),
        )?;
        let mut store = Self {
            database_path,
            database,
            cipher,
            _lock: lock,
        };
        store.ensure_schema(!database_existed)?;
        Ok(store)
    }

    fn column_family(&self, name: &str) -> NodeResult<&ColumnFamily> {
        self.database.cf_handle(name).ok_or_else(|| {
            invalid_state_database(
                &self.database_path,
                format!("required column family is missing: {name}"),
            )
        })
    }

    fn ensure_schema(&mut self, allow_initialize: bool) -> NodeResult<()> {
        let existing = self.get_decoded(CF_METADATA, META_SCHEMA_VERSION)?;
        match existing {
            None => {
                if !allow_initialize || self.database_contains_application_data()? {
                    return Err(invalid_state_database(
                        &self.database_path,
                        "schema metadata is missing from an existing or non-empty database",
                    ));
                }
                let mut batch = WriteBatch::default();
                self.put_encoded(
                    &mut batch,
                    CF_METADATA,
                    META_SCHEMA_VERSION,
                    ROCKS_SCHEMA_VERSION.to_string().as_bytes(),
                )?;
                self.put_encoded(&mut batch, CF_METADATA, META_BACKEND, b"rocksdb")?;
                self.put_encoded(
                    &mut batch,
                    CF_METADATA,
                    META_ENCRYPTION,
                    self.cipher.mode().as_bytes(),
                )?;
                self.put_encoded(
                    &mut batch,
                    CF_METADATA,
                    META_KEY_ID,
                    self.cipher.key_id().as_bytes(),
                )?;
                self.write_sync(batch)?;
            }
            Some(version) => {
                if version != ROCKS_SCHEMA_VERSION.to_string().as_bytes() {
                    return Err(invalid_state_database(
                        &self.database_path,
                        format!(
                            "unsupported RocksDB schema version: {}",
                            String::from_utf8_lossy(&version)
                        ),
                    ));
                }
                let backend = self
                    .get_decoded(CF_METADATA, META_BACKEND)?
                    .ok_or_else(|| {
                        invalid_state_database(&self.database_path, "missing backend metadata")
                    })?;
                if backend != b"rocksdb" {
                    return Err(invalid_state_database(
                        &self.database_path,
                        "backend metadata is not rocksdb",
                    ));
                }
                let stored_encryption = self
                    .get_decoded(CF_METADATA, META_ENCRYPTION)?
                    .ok_or_else(|| {
                        invalid_state_database(&self.database_path, "missing encryption metadata")
                    })?;
                let key_id = self
                    .get_decoded(CF_METADATA, META_KEY_ID)?
                    .ok_or_else(|| {
                        invalid_state_database(&self.database_path, "missing encryption key id")
                    })?;
                if stored_encryption == self.cipher.mode().as_bytes()
                    && key_id == self.cipher.key_id().as_bytes()
                {
                    // The configured cipher matches the persisted storage envelope.
                } else if stored_encryption == b"plaintext"
                    && key_id == b"none"
                    && matches!(self.cipher, ValueCipher::XChaCha20Poly1305 { .. })
                    && self.cipher.allow_plaintext_migration()
                {
                    self.migrate_plaintext_values_to_encrypted()?;
                } else {
                    return Err(invalid_state_database(
                        &self.database_path,
                        format!(
                            "storage encryption metadata mismatch: stored mode={} stored key_id={} configured mode={} configured key_id={}",
                            String::from_utf8_lossy(&stored_encryption),
                            String::from_utf8_lossy(&key_id),
                            self.cipher.mode(),
                            self.cipher.key_id()
                        ),
                    ));
                }
            }
        }
        self.cipher.disable_plaintext_migration();
        self.ensure_default_column_family_empty()?;
        Ok(())
    }

    fn database_contains_application_data(&self) -> NodeResult<bool> {
        if self
            .database
            .iterator_opt(IteratorMode::Start, checked_read_options())
            .next()
            .transpose()?
            .is_some()
        {
            return Ok(true);
        }
        for column_family in COLUMN_FAMILIES {
            let handle = self.column_family(column_family)?;
            if self
                .database
                .iterator_cf_opt(handle, checked_read_options(), IteratorMode::Start)
                .next()
                .transpose()?
                .is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn ensure_default_column_family_empty(&self) -> NodeResult<()> {
        if self
            .database
            .iterator_opt(IteratorMode::Start, checked_read_options())
            .next()
            .transpose()?
            .is_some()
        {
            return Err(invalid_state_database(
                &self.database_path,
                "default column family must remain empty",
            ));
        }
        Ok(())
    }

    fn migrate_plaintext_values_to_encrypted(&self) -> NodeResult<()> {
        if !matches!(self.cipher, ValueCipher::XChaCha20Poly1305 { .. }) {
            return Err(invalid_state_database(
                &self.database_path,
                "plaintext migration requires an authenticated storage cipher",
            ));
        }

        for column_family in COLUMN_FAMILIES {
            let handle = self.column_family(column_family)?;
            let mut batch = WriteBatch::default();
            let mut pending = 0_usize;
            for item in self.database.iterator_cf_opt(
                handle,
                checked_read_options(),
                IteratorMode::Start,
            ) {
                let (key, value) = item?;
                if let Some(plaintext) = value.strip_prefix(PLAINTEXT_VALUE_MAGIC) {
                    let encrypted = self.cipher.seal(column_family, &key, plaintext)?;
                    batch.put_cf(handle, &key, encrypted);
                    pending += 1;
                } else {
                    self.cipher.open(column_family, &key, &value)?;
                }
                if pending >= ENCRYPTION_MIGRATION_BATCH_SIZE {
                    self.write_sync(std::mem::take(&mut batch))?;
                    pending = 0;
                }
            }
            if pending > 0 {
                self.write_sync(batch)?;
            }
        }

        let mut metadata = WriteBatch::default();
        self.put_encoded(
            &mut metadata,
            CF_METADATA,
            META_ENCRYPTION,
            self.cipher.mode().as_bytes(),
        )?;
        self.put_encoded(
            &mut metadata,
            CF_METADATA,
            META_KEY_ID,
            self.cipher.key_id().as_bytes(),
        )?;
        self.write_sync(metadata)
    }

    fn get_decoded(&self, column_family: &str, key: &[u8]) -> NodeResult<Option<Vec<u8>>> {
        let handle = self.column_family(column_family)?;
        self.database
            .get_cf_opt(handle, key, &checked_read_options())?
            .map(|value| self.cipher.open(column_family, key, &value))
            .transpose()
    }

    fn put_encoded(
        &self,
        batch: &mut WriteBatch,
        column_family: &str,
        key: &[u8],
        value: &[u8],
    ) -> NodeResult<()> {
        let handle = self.column_family(column_family)?;
        let encoded = self.cipher.seal(column_family, key, value)?;
        batch.put_cf(handle, key, encoded);
        Ok(())
    }

    fn clear_column_family(
        &self,
        batch: &mut WriteBatch,
        column_family: &str,
    ) -> NodeResult<()> {
        let handle = self.column_family(column_family)?;
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, _) = item?;
            batch.delete_cf(handle, key);
        }
        Ok(())
    }

    fn write_sync(&self, batch: WriteBatch) -> NodeResult<()> {
        let mut options = WriteOptions::default();
        options.set_sync(true);
        options.disable_wal(false);
        self.database.write_opt(batch, &options)?;
        self.database.flush_wal(true)?;
        Ok(())
    }

    fn load_chain_state(&self) -> NodeResult<Option<PersistedChainState>> {
        self.get_decoded(CF_CANONICAL_STATE, STATE_SNAPSHOT_KEY)?
            .map(|value| self.decode_json(CF_CANONICAL_STATE, STATE_SNAPSHOT_KEY, &value))
            .transpose()
    }

    fn persist_chain(&self, blocks: &[Block], chain: &Chain) -> NodeResult<()> {
        let snapshot = persisted_chain_state(blocks, chain)?;
        let previous = self.load_chain_state()?;
        let incremental = previous.as_ref().is_some_and(|previous| {
            blocks.last().is_some_and(|tip| {
                previous.block_count.saturating_add(1) == snapshot.block_count
                    && hash_to_hex(&tip.header.previous_hash) == previous.tip_hash
            })
        });

        let mut batch = WriteBatch::default();
        if !incremental {
            self.clear_column_family(&mut batch, CF_ACCOUNTS)?;
            self.clear_column_family(&mut batch, CF_BLOCK_METADATA)?;
        }
        self.clear_column_family(&mut batch, CF_RECENT_TRANSACTIONS)?;

        self.put_json(
            &mut batch,
            CF_CANONICAL_STATE,
            STATE_SNAPSHOT_KEY,
            &snapshot,
        )?;

        if incremental {
            if let Some(block) = blocks.last() {
                self.put_block_metadata(&mut batch, block, chain.cumulative_work()?)?;
                for address in touched_addresses(block) {
                    self.put_account(&mut batch, chain.state(), &address)?;
                }
            }
        } else {
            let mut cumulative_work = 0_u128;
            for block in blocks {
                cumulative_work = cumulative_work
                    .checked_add(block_work(block)?)
                    .ok_or_else(|| NodeError::Input("chain work overflow".to_owned()))?;
                self.put_block_metadata(&mut batch, block, cumulative_work)?;
            }
            for address in all_state_addresses(chain.state()) {
                self.put_account(&mut batch, chain.state(), &address)?;
            }
        }

        for transaction_hash in chain.state().applied_transaction_hashes() {
            self.put_encoded(
                &mut batch,
                CF_RECENT_TRANSACTIONS,
                transaction_hash.as_bytes(),
                b"confirmed",
            )?;
        }
        self.write_sync(batch)
    }

    fn put_account(
        &self,
        batch: &mut WriteBatch,
        state: &LedgerState,
        address: &str,
    ) -> NodeResult<()> {
        let account = PersistedAccountState {
            balance_atomic: state.balance_of(address).as_atomic(),
            next_nonce: state.next_nonce_of(address),
        };
        self.put_json(batch, CF_ACCOUNTS, address.as_bytes(), &account)
    }

    fn put_block_metadata(
        &self,
        batch: &mut WriteBatch,
        block: &Block,
        cumulative_work: u128,
    ) -> NodeResult<()> {
        let metadata = persisted_block_metadata(block, cumulative_work)?;
        self.put_json(
            batch,
            CF_BLOCK_METADATA,
            &block.header.height.to_be_bytes(),
            &metadata,
        )
    }

    fn put_json<T: Serialize>(
        &self,
        batch: &mut WriteBatch,
        column_family: &str,
        key: &[u8],
        value: &T,
    ) -> NodeResult<()> {
        self.put_encoded(
            batch,
            column_family,
            key,
            &serde_json::to_vec(value)?,
        )
    }

    fn decode_json<T: for<'de> Deserialize<'de>>(
        &self,
        column_family: &str,
        key: &[u8],
        value: &[u8],
    ) -> NodeResult<T> {
        serde_json::from_slice(value).map_err(|error| {
            invalid_state_database(
                &self.database_path,
                format!(
                    "invalid JSON in column family {column_family} at key {}: {error}",
                    display_database_key(key)
                ),
            )
        })
    }

    fn load_mempool(&self, legacy_mempool_dir: &Path) -> NodeResult<Vec<PendingTransactionRecord>> {
        self.migrate_legacy_mempool_if_needed(legacy_mempool_dir)?;
        let mut records = self.load_mempool_records()?;
        records.sort_by(|left, right| {
            left.received_at_unix_seconds
                .cmp(&right.received_at_unix_seconds)
                .then_with(|| left.tx_hash.cmp(&right.tx_hash))
        });
        Ok(records)
    }

    fn migrate_legacy_mempool_if_needed(&self, legacy_mempool_dir: &Path) -> NodeResult<()> {
        if self
            .get_decoded(CF_METADATA, META_MEMPOOL_INITIALIZED)?
            .is_some()
        {
            return Ok(());
        }
        let legacy_path = legacy_mempool_dir.join(crate::mempool::MEMPOOL_FILE_NAME);
        let records = if legacy_path.exists() {
            let bytes = fs::read(&legacy_path)?;
            serde_json::from_slice::<Vec<PendingTransactionRecord>>(&bytes).map_err(|error| {
                NodeError::InvalidMempoolFile {
                    path: legacy_path,
                    message: error.to_string(),
                }
            })?
        } else {
            Vec::new()
        };
        self.replace_mempool(&records)
    }

    fn replace_mempool(&self, records: &[PendingTransactionRecord]) -> NodeResult<()> {
        validate_mempool_records(records)?;
        let mut batch = WriteBatch::default();
        self.clear_column_family(&mut batch, CF_MEMPOOL)?;
        for record in records {
            self.put_json(
                &mut batch,
                CF_MEMPOOL,
                record.tx_hash.as_bytes(),
                record,
            )?;
        }
        self.put_encoded(
            &mut batch,
            CF_METADATA,
            META_MEMPOOL_INITIALIZED,
            b"true",
        )?;
        self.write_sync(batch)
    }

    fn verify(&self, expected_network: Network, blocks: &[Block]) -> NodeResult<RocksStateStatus> {
        self.ensure_default_column_family_empty()?;
        self.count_and_authenticate(CF_METADATA)?;
        if self.count_and_authenticate(CF_CANONICAL_STATE)? != 1 {
            return Err(invalid_state_database(
                &self.database_path,
                "canonical state column family must contain exactly one snapshot",
            ));
        }
        let persisted = self.load_chain_state()?.ok_or_else(|| {
            invalid_state_database(&self.database_path, "canonical state snapshot is missing")
        })?;
        let replayed = Chain::from_blocks(expected_network, blocks.iter().cloned())?;
        let expected = persisted_chain_state(blocks, &replayed)?;
        if persisted != expected {
            return Err(invalid_state_database(
                &self.database_path,
                "persisted state does not match full replay oracle",
            ));
        }

        let expected_accounts = expected_account_map(replayed.state());
        let actual_accounts = self.load_accounts()?;
        if actual_accounts != expected_accounts {
            return Err(invalid_state_database(
                &self.database_path,
                "account column family does not match replayed ledger state",
            ));
        }

        let actual_block_metadata = self.load_block_metadata()?;
        let expected_block_metadata = expected_block_metadata(blocks)?;
        if actual_block_metadata != expected_block_metadata {
            return Err(invalid_state_database(
                &self.database_path,
                "block metadata does not match the fully replayed canonical chain",
            ));
        }
        let recent_transactions = self.load_recent_transactions()?;
        if recent_transactions != *replayed.state().applied_transaction_hashes() {
            return Err(invalid_state_database(
                &self.database_path,
                "recent transaction index does not match replayed ledger state",
            ));
        }
        let mempool_transactions = self.load_mempool_records()?;

        Ok(RocksStateStatus {
            schema_version: ROCKS_SCHEMA_VERSION,
            network_id: persisted.network_id,
            block_count: persisted.block_count,
            tip_height: persisted.tip_height,
            tip_hash: persisted.tip_hash,
            accounts: actual_accounts.len(),
            recent_transactions: recent_transactions.len(),
            mempool_transactions: mempool_transactions.len(),
            encryption: self.cipher.mode().to_owned(),
            key_id: self.cipher.key_id().to_owned(),
        })
    }

    fn restored_status(&self) -> NodeResult<RocksStateStatus> {
        self.ensure_default_column_family_empty()?;
        self.count_and_authenticate(CF_METADATA)?;
        if self.count_and_authenticate(CF_CANONICAL_STATE)? != 1 {
            return Err(invalid_state_database(
                &self.database_path,
                "restored canonical state column family must contain exactly one snapshot",
            ));
        }
        let snapshot = self.load_chain_state()?.ok_or_else(|| {
            invalid_state_database(
                &self.database_path,
                "restored canonical state snapshot is missing",
            )
        })?;
        validate_restored_snapshot(&self.database_path, &snapshot)?;

        let accounts = self.load_accounts()?;
        if accounts != expected_account_map(&snapshot.state) {
            return Err(invalid_state_database(
                &self.database_path,
                "restored account column family does not match its canonical state snapshot",
            ));
        }

        let block_metadata = self.load_block_metadata()?;
        validate_restored_block_metadata(&self.database_path, &snapshot, &block_metadata)?;

        let recent_transactions = self.load_recent_transactions()?;
        if recent_transactions != *snapshot.state.applied_transaction_hashes() {
            return Err(invalid_state_database(
                &self.database_path,
                "restored recent transaction index does not match its canonical state snapshot",
            ));
        }
        let mempool_transactions = self.load_mempool_records()?;

        Ok(RocksStateStatus {
            schema_version: ROCKS_SCHEMA_VERSION,
            network_id: snapshot.network_id,
            block_count: snapshot.block_count,
            tip_height: snapshot.tip_height,
            tip_hash: snapshot.tip_hash,
            accounts: accounts.len(),
            recent_transactions: recent_transactions.len(),
            mempool_transactions: mempool_transactions.len(),
            encryption: self.cipher.mode().to_owned(),
            key_id: self.cipher.key_id().to_owned(),
        })
    }

    fn load_accounts(&self) -> NodeResult<BTreeMap<String, PersistedAccountState>> {
        let handle = self.column_family(CF_ACCOUNTS)?;
        let mut accounts = BTreeMap::new();
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, value) = item?;
            let address = String::from_utf8(key.to_vec()).map_err(|_| {
                invalid_state_database(&self.database_path, "account key is not valid UTF-8")
            })?;
            let decoded = self.cipher.open(CF_ACCOUNTS, &key, &value)?;
            let account = self.decode_json(CF_ACCOUNTS, &key, &decoded)?;
            if accounts.insert(address, account).is_some() {
                return Err(invalid_state_database(
                    &self.database_path,
                    "duplicate account key",
                ));
            }
        }
        Ok(accounts)
    }

    fn load_block_metadata(&self) -> NodeResult<BTreeMap<u64, PersistedBlockMetadata>> {
        let handle = self.column_family(CF_BLOCK_METADATA)?;
        let mut entries = BTreeMap::new();
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, value) = item?;
            let key_bytes: [u8; 8] = key.as_ref().try_into().map_err(|_| {
                invalid_state_database(
                    &self.database_path,
                    "block metadata key is not an eight-byte height",
                )
            })?;
            let height = u64::from_be_bytes(key_bytes);
            let decoded = self.cipher.open(CF_BLOCK_METADATA, &key, &value)?;
            let metadata = self.decode_json(CF_BLOCK_METADATA, &key, &decoded)?;
            if entries.insert(height, metadata).is_some() {
                return Err(invalid_state_database(
                    &self.database_path,
                    format!("duplicate block metadata height {height}"),
                ));
            }
        }
        Ok(entries)
    }

    fn load_recent_transactions(&self) -> NodeResult<BTreeSet<String>> {
        let handle = self.column_family(CF_RECENT_TRANSACTIONS)?;
        let mut transactions = BTreeSet::new();
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, value) = item?;
            let transaction_hash = String::from_utf8(key.to_vec()).map_err(|_| {
                invalid_state_database(
                    &self.database_path,
                    "recent transaction key is not valid UTF-8",
                )
            })?;
            if !is_canonical_hash(&transaction_hash) {
                return Err(invalid_state_database(
                    &self.database_path,
                    format!("invalid recent transaction hash key: {transaction_hash}"),
                ));
            }
            let decoded = self
                .cipher
                .open(CF_RECENT_TRANSACTIONS, &key, &value)?;
            if decoded != b"confirmed" {
                return Err(invalid_state_database(
                    &self.database_path,
                    format!(
                        "unexpected recent transaction marker for hash {transaction_hash}"
                    ),
                ));
            }
            transactions.insert(transaction_hash);
        }
        Ok(transactions)
    }

    fn load_mempool_records(&self) -> NodeResult<Vec<PendingTransactionRecord>> {
        let handle = self.column_family(CF_MEMPOOL)?;
        let mut records = Vec::new();
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, value) = item?;
            let stored_hash = String::from_utf8(key.to_vec()).map_err(|_| {
                invalid_state_database(&self.database_path, "mempool key is not valid UTF-8")
            })?;
            let decoded = self.cipher.open(CF_MEMPOOL, &key, &value)?;
            let record: PendingTransactionRecord =
                self.decode_json(CF_MEMPOOL, &key, &decoded)?;
            validate_mempool_record(&record).map_err(|message| {
                invalid_state_database(&self.database_path, format!("invalid mempool record: {message}"))
            })?;
            if stored_hash != record.tx_hash {
                return Err(invalid_state_database(
                    &self.database_path,
                    format!(
                        "mempool key {stored_hash} does not match record hash {}",
                        record.tx_hash
                    ),
                ));
            }
            records.push(record);
        }
        Ok(records)
    }

    fn count_and_authenticate(&self, column_family: &str) -> NodeResult<usize> {
        let handle = self.column_family(column_family)?;
        let mut count = 0_usize;
        for item in self.database.iterator_cf_opt(
            handle,
            checked_read_options(),
            IteratorMode::Start,
        ) {
            let (key, value) = item?;
            self.cipher.open(column_family, &key, &value)?;
            count = count.saturating_add(1);
        }
        Ok(count)
    }
}

pub fn state_database_path(data_dir: &Path) -> PathBuf {
    data_dir.join(STATE_DATABASE_DIR_NAME)
}

pub fn load_persisted_chain_state(
    data_dir: &Path,
    network: Network,
    expected_height: u64,
    expected_hash: &str,
) -> NodeResult<Option<LedgerState>> {
    let database_path = state_database_path(data_dir);
    match fs::symlink_metadata(&database_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(NodeError::Input(format!(
                "RocksDB state database is not a safe directory: {}",
                database_path.display()
            )))
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let store = RocksStateStore::open_existing(data_dir)?;
    let Some(snapshot) = store.load_chain_state()? else {
        return Ok(None);
    };
    if snapshot.schema_version != ROCKS_SCHEMA_VERSION
        || snapshot.network_id != network.network_id()
        || snapshot.tip_height != expected_height
        || snapshot.tip_hash != expected_hash
    {
        return Ok(None);
    }
    Ok(Some(snapshot.state))
}

pub fn persist_chain_state(data_dir: &Path, blocks: &[Block], chain: &Chain) -> NodeResult<()> {
    RocksStateStore::open(data_dir)?.persist_chain(blocks, chain)
}

pub fn load_persisted_mempool(
    data_dir: &Path,
    legacy_mempool_dir: &Path,
) -> NodeResult<Vec<PendingTransactionRecord>> {
    RocksStateStore::open(data_dir)?.load_mempool(legacy_mempool_dir)
}

pub fn replace_persisted_mempool(
    data_dir: &Path,
    records: &[PendingTransactionRecord],
) -> NodeResult<()> {
    RocksStateStore::open(data_dir)?.replace_mempool(records)
}

pub fn verify_state_database(
    data_dir: &Path,
    network: Network,
    blocks: &[Block],
) -> NodeResult<RocksStateStatus> {
    RocksStateStore::open_existing(data_dir)?.verify(network, blocks)
}

pub fn backup_state_database(
    data_dir: &Path,
    backup_repository: &Path,
    backups_to_keep: usize,
) -> NodeResult<RocksBackupInfo> {
    ensure_directory(data_dir, false, "RocksDB data directory")?;
    ensure_directory(
        backup_repository,
        true,
        "RocksDB incremental backup repository",
    )?;
    ensure_separate_storage_paths(data_dir, backup_repository)?;
    let store = RocksStateStore::open_existing(data_dir)?;
    let status = store.restored_status()?;
    store.database.flush_wal(true)?;
    let mut options = BackupEngineOptions::new(backup_repository)?;
    options.set_sync(true);
    options.set_max_background_operations(2);
    let environment = Env::new()?;
    let mut engine = BackupEngine::open(&options, &environment)?;
    engine.create_new_backup_flush(&store.database, true)?;
    if backups_to_keep > 0 {
        engine.purge_old_backups(backups_to_keep)?;
    }
    let info = engine
        .get_backup_info()
        .into_iter()
        .max_by_key(|item| item.backup_id)
        .ok_or_else(|| NodeError::Input("RocksDB backup engine returned no backup".to_owned()))?;
    engine.verify_backup(info.backup_id)?;
    Ok(RocksBackupInfo {
        backup_id: info.backup_id,
        timestamp: info.timestamp,
        size: info.size,
        files: info.num_files,
        network_id: status.network_id,
        block_count: status.block_count,
        tip_height: status.tip_height,
        tip_hash: status.tip_hash,
        encryption: status.encryption,
        key_id: status.key_id,
    })
}

pub fn restore_latest_state_database(
    data_dir: &Path,
    backup_repository: &Path,
) -> NodeResult<RocksStateStatus> {
    ensure_directory(data_dir, true, "RocksDB restore data directory")?;
    ensure_directory(
        backup_repository,
        false,
        "RocksDB incremental backup repository",
    )?;
    ensure_separate_storage_paths(data_dir, backup_repository)?;
    let database_path = state_database_path(data_dir);
    match fs::symlink_metadata(&database_path) {
        Ok(_) => {
            return Err(NodeError::Input(format!(
                "restore destination already contains RocksDB state: {}",
                database_path.display()
            )))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let lock = acquire_exclusive_lock(
        &data_dir.join(STATE_DATABASE_LOCK_FILE_NAME),
        "RocksDB restore destination",
    )?;
    let stage_root = unique_restore_stage_root(data_dir)?;
    fs::create_dir(&stage_root)?;
    let stage_database = state_database_path(&stage_root);

    let restore_result = (|| -> NodeResult<RocksStateStatus> {
        let mut options = BackupEngineOptions::new(backup_repository)?;
        options.set_sync(true);
        let environment = Env::new()?;
        let mut engine = BackupEngine::open(&options, &environment)?;
        let latest = engine
            .get_backup_info()
            .into_iter()
            .max_by_key(|item| item.backup_id)
            .ok_or_else(|| NodeError::Input("RocksDB backup repository is empty".to_owned()))?;
        engine.verify_backup(latest.backup_id)?;
        let restore_options = RestoreOptions::default();
        engine.restore_from_backup(
            &stage_database,
            &stage_database,
            &restore_options,
            latest.backup_id,
        )?;
        let stage_store = RocksStateStore::open_existing(&stage_root)?;
        let status = stage_store.restored_status()?;
        drop(stage_store);
        let stage_lock = stage_root.join(STATE_DATABASE_LOCK_FILE_NAME);
        if stage_lock.exists() {
            fs::remove_file(stage_lock)?;
        }
        fs::rename(&stage_database, &database_path)?;
        let _ = fs::remove_dir(&stage_root);
        Ok(status)
    })();
    drop(lock);
    if restore_result.is_err() && stage_root.exists() {
        fs::remove_dir_all(&stage_root)?;
    }
    restore_result
}

fn persisted_chain_state(blocks: &[Block], chain: &Chain) -> NodeResult<PersistedChainState> {
    let tip = blocks
        .last()
        .ok_or_else(|| NodeError::Input("cannot persist an empty canonical chain".to_owned()))?;
    if chain.blocks() != blocks {
        return Err(NodeError::Input(
            "cannot persist state for a different canonical block sequence".to_owned(),
        ));
    }
    Ok(PersistedChainState {
        schema_version: ROCKS_SCHEMA_VERSION,
        network_id: chain.network().network_id().to_owned(),
        block_count: u64::try_from(blocks.len())
            .map_err(|_| NodeError::Input("block count exceeds u64".to_owned()))?,
        tip_height: tip.header.height,
        tip_hash: hash_to_hex(&tip.hash()?),
        tip_timestamp: tip.header.timestamp,
        state: chain.state().clone(),
    })
}

fn expected_account_map(state: &LedgerState) -> BTreeMap<String, PersistedAccountState> {
    all_state_addresses(state)
        .into_iter()
        .map(|address| {
            let account = PersistedAccountState {
                balance_atomic: state.balance_of(&address).as_atomic(),
                next_nonce: state.next_nonce_of(&address),
            };
            (address, account)
        })
        .collect()
}

fn all_state_addresses(state: &LedgerState) -> BTreeSet<String> {
    state
        .balances()
        .keys()
        .chain(state.nonces().keys())
        .cloned()
        .collect()
}

fn touched_addresses(block: &Block) -> BTreeSet<String> {
    block
        .transactions
        .iter()
        .flat_map(|transaction| {
            transaction
                .from
                .iter()
                .chain(std::iter::once(&transaction.to))
        })
        .cloned()
        .collect()
}

fn ensure_directory(path: &Path, create_if_missing: bool, label: &str) -> NodeResult<()> {
    validate_path_components(path, label)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(NodeError::Input(format!(
            "{label} must not be a symlink: {}",
            path.display()
        ))),
        Ok(metadata) if !metadata.is_dir() => Err(NodeError::Input(format!(
            "{label} is not a directory: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound && create_if_missing => {
            fs::create_dir_all(path)?;
            validate_path_components(path, label)?;
            let metadata = fs::symlink_metadata(path)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(NodeError::Input(format!(
                    "{label} is not a safe directory: {}",
                    path.display()
                )));
            }
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(NodeError::Input(format!(
            "{label} does not exist: {}",
            path.display()
        ))),
        Err(error) => Err(error.into()),
    }
}

fn validate_path_components(path: &Path, label: &str) -> NodeResult<()> {
    if path.as_os_str().is_empty() {
        return Err(NodeError::Input(format!("{label} path must not be empty")));
    }

    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(NodeError::Input(format!(
                    "{label} path must not contain parent traversal: {}",
                    path.display()
                )))
            }
            Component::CurDir => continue,
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(NodeError::Input(format!(
                    "{label} path contains a symlink component: {}",
                    current.display()
                )))
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn validate_database_location(
    database_path: &Path,
    create_if_missing: bool,
) -> NodeResult<bool> {
    validate_path_components(database_path, "RocksDB state database")?;
    match fs::symlink_metadata(database_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(NodeError::Input(format!(
                "RocksDB state database must be a real directory: {}",
                database_path.display()
            )))
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound && create_if_missing => return Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(NodeError::Input(format!(
                "RocksDB state database does not exist: {}",
                database_path.display()
            )))
        }
        Err(error) => return Err(error.into()),
    }

    let current_path = database_path.join("CURRENT");
    match fs::symlink_metadata(&current_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(NodeError::Input(format!(
                "RocksDB CURRENT marker is not a regular file: {}",
                current_path.display()
            )))
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound && create_if_missing => {
            if fs::read_dir(database_path)?.next().transpose()?.is_some() {
                return Err(NodeError::Input(format!(
                    "RocksDB directory exists without CURRENT and is not empty: {}",
                    database_path.display()
                )));
            }
            Ok(false)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(NodeError::Input(format!(
            "RocksDB CURRENT marker is missing: {}",
            current_path.display()
        ))),
        Err(error) => Err(error.into()),
    }
}

fn acquire_exclusive_lock(path: &Path, label: &str) -> NodeResult<File> {
    validate_path_components(path, label)?;
    if let Some(parent) = path.parent() {
        ensure_directory(parent, false, label)?;
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(NodeError::Input(format!(
                "{label} lock is not a regular file: {}",
                path.display()
            )))
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NodeError::Input(format!(
            "{label} lock became unsafe while opening: {}",
            path.display()
        )));
    }

    let deadline = Instant::now() + LOCK_WAIT_TIMEOUT;
    loop {
        match FileExt::try_lock_exclusive(&lock) {
            Ok(()) => return Ok(lock),
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(LOCK_RETRY_INTERVAL);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                return Err(NodeError::Input(format!(
                    "timed out waiting for {label} lock: {}",
                    path.display()
                )))
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn ensure_separate_storage_paths(data_dir: &Path, backup_repository: &Path) -> NodeResult<()> {
    let canonical_data = fs::canonicalize(data_dir)?;
    let canonical_backup = fs::canonicalize(backup_repository)?;
    if canonical_backup.starts_with(&canonical_data)
        || canonical_data.starts_with(&canonical_backup)
    {
        return Err(NodeError::Input(format!(
            "RocksDB backup repository and data directory must be disjoint: data={} backup={}",
            canonical_data.display(),
            canonical_backup.display()
        )));
    }
    Ok(())
}

fn unique_restore_stage_root(data_dir: &Path) -> NodeResult<PathBuf> {
    for _ in 0..32 {
        let mut random = [0_u8; 8];
        OsRng.try_fill_bytes(&mut random).map_err(|error| {
            NodeError::Input(format!(
                "operating-system randomness failed while creating restore stage: {error}"
            ))
        })?;
        let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let candidate = data_dir.join(format!(
            "{RESTORE_STAGE_PREFIX}{}-{suffix}",
            std::process::id()
        ));
        match fs::symlink_metadata(&candidate) {
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(candidate),
            Err(error) => return Err(error.into()),
        }
    }
    Err(NodeError::Input(
        "could not allocate a unique RocksDB restore stage".to_owned(),
    ))
}

fn checked_read_options() -> ReadOptions {
    let mut options = ReadOptions::default();
    options.set_verify_checksums(true);
    options.fill_cache(false);
    options
}

fn persisted_block_metadata(
    block: &Block,
    cumulative_work: u128,
) -> NodeResult<PersistedBlockMetadata> {
    Ok(PersistedBlockMetadata {
        height: block.header.height,
        hash: hash_to_hex(&block.hash()?),
        previous_hash: hash_to_hex(&block.header.previous_hash),
        network_id: block.header.network_id.clone(),
        timestamp: block.header.timestamp,
        difficulty_leading_zero_bits: block.header.difficulty_leading_zero_bits,
        transaction_count: block.transactions.len(),
        cumulative_work: cumulative_work.to_string(),
    })
}

fn expected_block_metadata(
    blocks: &[Block],
) -> NodeResult<BTreeMap<u64, PersistedBlockMetadata>> {
    let mut entries = BTreeMap::new();
    let mut cumulative_work = 0_u128;
    for block in blocks {
        cumulative_work = cumulative_work
            .checked_add(block_work(block)?)
            .ok_or_else(|| NodeError::Input("chain work overflow".to_owned()))?;
        let metadata = persisted_block_metadata(block, cumulative_work)?;
        if entries.insert(block.header.height, metadata).is_some() {
            return Err(NodeError::Input(format!(
                "duplicate canonical block height {}",
                block.header.height
            )));
        }
    }
    Ok(entries)
}

fn validate_restored_snapshot(
    database_path: &Path,
    snapshot: &PersistedChainState,
) -> NodeResult<()> {
    if snapshot.schema_version != ROCKS_SCHEMA_VERSION {
        return Err(invalid_state_database(
            database_path,
            format!(
                "restored snapshot schema version {} is unsupported",
                snapshot.schema_version
            ),
        ));
    }
    if Network::from_network_id(&snapshot.network_id).is_none() {
        return Err(invalid_state_database(
            database_path,
            format!("restored snapshot network is invalid: {}", snapshot.network_id),
        ));
    }
    if snapshot.block_count == 0
        || snapshot.tip_height.checked_add(1) != Some(snapshot.block_count)
    {
        return Err(invalid_state_database(
            database_path,
            "restored snapshot block count and tip height are inconsistent",
        ));
    }
    if !is_canonical_hash(&snapshot.tip_hash) {
        return Err(invalid_state_database(
            database_path,
            "restored snapshot tip hash is not canonical",
        ));
    }
    if snapshot.state.applied_block_height() != Some(snapshot.tip_height)
        || snapshot
            .state
            .tip_hash()
            .is_none_or(|hash| hash_to_hex(&hash) != snapshot.tip_hash)
        || snapshot.state.tip_timestamp() != Some(snapshot.tip_timestamp)
    {
        return Err(invalid_state_database(
            database_path,
            "restored ledger state does not match snapshot tip metadata",
        ));
    }
    Ok(())
}

fn validate_restored_block_metadata(
    database_path: &Path,
    snapshot: &PersistedChainState,
    entries: &BTreeMap<u64, PersistedBlockMetadata>,
) -> NodeResult<()> {
    if u64::try_from(entries.len()).ok() != Some(snapshot.block_count) {
        return Err(invalid_state_database(
            database_path,
            "restored block metadata count does not match snapshot",
        ));
    }

    let mut previous_hash = "0".repeat(64);
    let mut cumulative_work = 0_u128;
    for height in 0..snapshot.block_count {
        let metadata = entries.get(&height).ok_or_else(|| {
            invalid_state_database(
                database_path,
                format!("restored block metadata is missing height {height}"),
            )
        })?;
        if metadata.height != height
            || metadata.network_id != snapshot.network_id
            || metadata.previous_hash != previous_hash
            || !is_canonical_hash(&metadata.hash)
            || !is_canonical_hash(&metadata.previous_hash)
            || metadata.transaction_count == 0
        {
            return Err(invalid_state_database(
                database_path,
                format!("restored block metadata is invalid at height {height}"),
            ));
        }
        let work = 1_u128
            .checked_shl(u32::from(metadata.difficulty_leading_zero_bits))
            .ok_or_else(|| {
                invalid_state_database(
                    database_path,
                    format!("restored difficulty overflows work at height {height}"),
                )
            })?;
        cumulative_work = cumulative_work.checked_add(work).ok_or_else(|| {
            invalid_state_database(database_path, "restored cumulative work overflow")
        })?;
        if metadata.cumulative_work.parse::<u128>().ok() != Some(cumulative_work) {
            return Err(invalid_state_database(
                database_path,
                format!("restored cumulative work is invalid at height {height}"),
            ));
        }
        previous_hash.clone_from(&metadata.hash);
    }

    let tip = entries.get(&snapshot.tip_height).ok_or_else(|| {
        invalid_state_database(database_path, "restored tip block metadata is missing")
    })?;
    if tip.hash != snapshot.tip_hash || tip.timestamp != snapshot.tip_timestamp {
        return Err(invalid_state_database(
            database_path,
            "restored tip block metadata does not match snapshot",
        ));
    }
    Ok(())
}

fn validate_mempool_records(records: &[PendingTransactionRecord]) -> NodeResult<()> {
    let mut hashes = BTreeSet::new();
    for record in records {
        validate_mempool_record(record).map_err(NodeError::Input)?;
        if !hashes.insert(record.tx_hash.clone()) {
            return Err(NodeError::Input(format!(
                "duplicate mempool transaction hash {}",
                record.tx_hash
            )));
        }
    }
    Ok(())
}

fn validate_mempool_record(record: &PendingTransactionRecord) -> Result<(), String> {
    if !is_canonical_hash(&record.tx_hash) {
        return Err(format!(
            "transaction hash is not canonical: {}",
            record.tx_hash
        ));
    }
    if record.transaction.is_coinbase() {
        return Err("coinbase transaction cannot be persisted in the mempool".to_owned());
    }
    let actual_hash = tx_hash_string(&record.transaction);
    if actual_hash != record.tx_hash {
        return Err(format!(
            "transaction hash mismatch: stored={} actual={actual_hash}",
            record.tx_hash
        ));
    }
    Ok(())
}

fn is_canonical_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn display_database_key(key: &[u8]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn database_options(create_if_missing: bool) -> Options {
    let mut options = Options::default();
    options.create_if_missing(create_if_missing);
    options.create_missing_column_families(create_if_missing);
    options.set_atomic_flush(true);
    options.set_bytes_per_sync(1 << 20);
    options.set_max_background_jobs(2);
    options.set_max_open_files(256);
    options.set_max_subcompactions(1);
    options.set_paranoid_checks(true);
    options.set_use_fsync(true);
    options.set_wal_recovery_mode(DBRecoveryMode::AbsoluteConsistency);
    options
}

fn column_family_options() -> Options {
    let mut block_options = BlockBasedOptions::default();
    block_options.set_bloom_filter(10.0, false);
    let mut options = Options::default();
    options.set_block_based_table_factory(&block_options);
    options.set_compression_type(DBCompressionType::Lz4);
    options.set_bottommost_compression_type(DBCompressionType::Lz4);
    options.set_level_compaction_dynamic_level_bytes(true);
    options.set_max_bytes_for_level_base(256 * 1024 * 1024);
    options.set_max_write_buffer_number(2);
    options.set_optimize_filters_for_hits(true);
    options.set_target_file_size_base(64 * 1024 * 1024);
    options.set_write_buffer_size(32 * 1024 * 1024);
    options
}

fn reject_unknown_column_families(database_path: &Path) -> NodeResult<()> {
    if !database_path.join("CURRENT").exists() {
        return Ok(());
    }
    let known: BTreeSet<&str> = std::iter::once("default")
        .chain(COLUMN_FAMILIES)
        .collect();
    let actual = DB::list_cf(&Options::default(), database_path)?;
    let unknown: Vec<String> = actual
        .into_iter()
        .filter(|name| !known.contains(name.as_str()))
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(invalid_state_database(
            database_path,
            format!("unknown column families: {}", unknown.join(", ")),
        ))
    }
}

fn read_storage_key(path: &Path) -> NodeResult<Zeroizing<[u8; 32]>> {
    let bytes = fs::read(path)?;
    if bytes.len() == 32 {
        let mut key = [0_u8; 32];
        key.copy_from_slice(&bytes);
        return Ok(Zeroizing::new(key));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| {
            NodeError::Input(format!(
                "storage key is not valid UTF-8: {}",
                path.display()
            ))
        })?
        .trim();
    if text.len() != 64 {
        return Err(NodeError::Input(format!(
            "storage key must contain 32 raw bytes or 64 hexadecimal characters: {}",
            path.display()
        )));
    }
    let mut key = [0_u8; 32];
    for (index, chunk) in text.as_bytes().chunks_exact(2).enumerate() {
        key[index] = (decode_hex_nibble(chunk[0])? << 4) | decode_hex_nibble(chunk[1])?;
    }
    Ok(Zeroizing::new(key))
}

fn decode_hex_nibble(value: u8) -> NodeResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(NodeError::Input(
            "storage key contains a non-hexadecimal character".to_owned(),
        )),
    }
}

fn environment_flag(name: &str) -> NodeResult<bool> {
    match std::env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" | "" => Ok(false),
            _ => Err(NodeError::Input(format!(
                "{name} must be true or false"
            ))),
        },
        Err(std::env::VarError::NotPresent) => Ok(false),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(NodeError::Input(format!("{name} is not valid Unicode")))
        }
    }
}

fn value_aad(column_family: &str, key: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(32 + column_family.len() + key.len());
    aad.extend_from_slice(b"alvenqis-rocksdb-value-v1");
    aad.push(0);
    aad.extend_from_slice(column_family.as_bytes());
    aad.push(0);
    aad.extend_from_slice(key);
    aad
}

fn invalid_state_database(path: &Path, message: impl Into<String>) -> NodeError {
    NodeError::InvalidChainDatabase {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alvenqis_core::{
        devnet_genesis, Address, Amount, PrivateKey, FIRST_ACCOUNT_NONCE,
    };

    fn miner_address() -> String {
        Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::Devnet,
        )
        .to_string()
    }

    #[test]
    fn encrypted_value_roundtrip_authenticates_column_family_and_key() {
        let cipher = ValueCipher::encrypted_for_test([7_u8; 32]);
        let encoded = cipher
            .seal(CF_CANONICAL_STATE, b"snapshot", b"ledger")
            .expect("seal");
        assert_ne!(encoded, b"ledger");
        assert_eq!(
            cipher
                .open(CF_CANONICAL_STATE, b"snapshot", &encoded)
                .expect("open"),
            b"ledger"
        );
        assert!(cipher
            .open(CF_CANONICAL_STATE, b"different", &encoded)
            .is_err());
    }

    #[test]
    fn persisted_state_matches_full_replay_oracle() {
        let temp = tempfile::tempdir().expect("temp");
        let data_dir = temp.path().join(".alvenqis-dev/chain");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let blocks = vec![genesis];
        let chain = Chain::from_blocks(Network::Devnet, blocks.clone()).expect("chain");
        let store = RocksStateStore::open_with_cipher(
            &data_dir,
            ValueCipher::encrypted_for_test([11_u8; 32]),
        )
        .expect("open");
        store.persist_chain(&blocks, &chain).expect("persist");
        let status = store
            .verify(Network::Devnet, &blocks)
            .expect("verify against replay");
        assert_eq!(status.block_count, 1);
        assert_eq!(status.accounts, 1);
        assert_eq!(status.encryption, "xchacha20poly1305");
    }

    #[test]
    fn incremental_backup_restores_into_empty_destination() {
        let temp = tempfile::tempdir().expect("temp");
        let source = temp.path().join(".alvenqis-dev/source");
        let destination = temp.path().join(".alvenqis-dev/restored");
        let backup = temp.path().join("backups/rocksdb");
        let genesis = devnet_genesis(&miner_address()).expect("genesis");
        let blocks = vec![genesis];
        let chain = Chain::from_blocks(Network::Devnet, blocks.clone()).expect("chain");
        RocksStateStore::open_with_cipher(
            &source,
            ValueCipher::Plaintext {
                allow_plaintext_migration: false,
            },
        )
        .expect("open")
        .persist_chain(&blocks, &chain)
        .expect("persist");

        let info = backup_state_database(&source, &backup, 3).expect("backup");
        assert!(info.backup_id > 0);
        let restored = restore_latest_state_database(&destination, &backup).expect("restore");
        assert_eq!(restored.tip_height, 0);
        assert_eq!(restored.block_count, 1);
    }

    #[test]
    fn account_defaults_match_ledger_contract() {
        let state = LedgerState::new();
        assert_eq!(state.balance_of("missing"), Amount::ZERO);
        assert_eq!(state.next_nonce_of("missing"), FIRST_ACCOUNT_NONCE);
    }
}
