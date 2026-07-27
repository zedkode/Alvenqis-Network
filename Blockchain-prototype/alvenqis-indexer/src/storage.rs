//! Crash-safe indexer storage.
//!
//! Primary format: SQLite (`index.sqlite3`) with indexed tables for blocks,
//! transactions, and address activity. Legacy `index.json` is imported once on
//! first open and left in place as a read-only rollback artifact.

use crate::error::{IndexerError, IndexerResult};
use crate::index::{AddressActivity, IndexData, IndexSummary, IndexedBlock, IndexedTransaction};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const INDEX_FILE_NAME: &str = "index.json";
pub const INDEX_DB_FILE_NAME: &str = "index.sqlite3";

pub fn index_file_path(index_dir: &Path) -> PathBuf {
    index_dir.join(INDEX_FILE_NAME)
}

pub fn index_db_path(index_dir: &Path) -> PathBuf {
    index_dir.join(INDEX_DB_FILE_NAME)
}

pub fn ensure_index_dir(index_dir: &Path) -> IndexerResult<()> {
    fs::create_dir_all(index_dir)?;
    Ok(())
}

fn open_db(index_dir: &Path) -> IndexerResult<Connection> {
    ensure_index_dir(index_dir)?;
    let path = index_db_path(index_dir);
    let connection = Connection::open(&path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    // The explorer index is rebuildable from the consensus database. NORMAL keeps
    // WAL durability while avoiding an fsync for every page on resource-bound VPSes.
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "cache_size", -32_768_i64)?;
    connection.pragma_update(None, "wal_autocheckpoint", 1_000_i64)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    initialize_schema(&connection)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> IndexerResult<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS blocks (
            height INTEGER PRIMARY KEY NOT NULL,
            hash TEXT NOT NULL UNIQUE,
            payload TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash);
        CREATE TABLE IF NOT EXISTS transactions (
            hash TEXT PRIMARY KEY NOT NULL,
            block_height INTEGER NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tx_height ON transactions(block_height);
        CREATE TABLE IF NOT EXISTS addresses (
            address TEXT PRIMARY KEY NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS block_rewards (
            height INTEGER PRIMARY KEY NOT NULL,
            miner_reward_atomic INTEGER NOT NULL,
            fees_atomic INTEGER NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// Atomic write: full replace of index contents inside a single SQLite transaction.
pub fn write_index(index_dir: &Path, index: &IndexData) -> IndexerResult<()> {
    let mut connection = open_db(index_dir)?;
    let tx = connection.transaction()?;
    tx.execute_batch(
        "DELETE FROM meta; DELETE FROM blocks; DELETE FROM transactions; DELETE FROM addresses; DELETE FROM block_rewards;",
    )?;

    let summary_json = serde_json::to_string(&index.summary)?;
    tx.execute(
        "INSERT INTO meta(key, value) VALUES ('summary', ?1)",
        params![summary_json],
    )?;

    for (height, block) in &index.blocks_by_height {
        let payload = serde_json::to_string(block)?;
        tx.execute(
            "INSERT INTO blocks(height, hash, payload) VALUES (?1, ?2, ?3)",
            params![*height as i64, block.hash, payload],
        )?;
    }
    for (hash, indexed_tx) in &index.transactions_by_hash {
        let payload = serde_json::to_string(indexed_tx)?;
        tx.execute(
            "INSERT INTO transactions(hash, block_height, payload) VALUES (?1, ?2, ?3)",
            params![hash, indexed_tx.block_height as i64, payload],
        )?;
    }
    for (address, activity) in &index.addresses {
        let payload = serde_json::to_string(activity)?;
        tx.execute(
            "INSERT INTO addresses(address, payload) VALUES (?1, ?2)",
            params![address, payload],
        )?;
    }
    for (height, reward) in &index.miner_rewards_by_block {
        let fees = index.fees_by_block.get(height).copied().unwrap_or(0);
        tx.execute(
            "INSERT INTO block_rewards(height, miner_reward_atomic, fees_atomic) VALUES (?1, ?2, ?3)",
            params![*height as i64, *reward as i64, fees as i64],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Load full index for bulk operations. Prefers SQLite; migrates legacy JSON once.
pub fn load_index(index_dir: &Path) -> IndexerResult<IndexData> {
    ensure_index_dir(index_dir)?;
    let db_path = index_db_path(index_dir);
    let json_path = index_file_path(index_dir);

    if db_path.exists() {
        return load_index_from_db(index_dir);
    }

    if json_path.exists() {
        let legacy = load_index_from_json(&json_path)?;
        // One-time import into SQLite; keep index.json as rollback artifact.
        write_index(index_dir, &legacy)?;
        return Ok(legacy);
    }

    Err(IndexerError::IndexNotInitialized(json_path))
}

fn load_index_from_json(path: &Path) -> IndexerResult<IndexData> {
    let file = fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|error| IndexerError::InvalidIndexFile {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn load_index_from_db(index_dir: &Path) -> IndexerResult<IndexData> {
    let connection = open_db(index_dir)?;
    let summary_json: String = connection
        .query_row("SELECT value FROM meta WHERE key = 'summary'", [], |row| {
            row.get(0)
        })
        .map_err(|_| IndexerError::InvalidIndexFile {
            path: index_db_path(index_dir),
            message: "missing summary meta".to_owned(),
        })?;
    let summary: IndexSummary = serde_json::from_str(&summary_json)?;

    let mut blocks_by_height = BTreeMap::new();
    let mut blocks_by_hash = BTreeMap::new();
    let mut stmt = connection.prepare("SELECT height, hash, payload FROM blocks")?;
    let rows = stmt.query_map([], |row| {
        let height: i64 = row.get(0)?;
        let hash: String = row.get(1)?;
        let payload: String = row.get(2)?;
        Ok((height as u64, hash, payload))
    })?;
    for row in rows {
        let (height, hash, payload) = row?;
        let block: IndexedBlock = serde_json::from_str(&payload)?;
        blocks_by_hash.insert(hash, block.clone());
        blocks_by_height.insert(height, block);
    }

    let mut transactions_by_hash = BTreeMap::new();
    let mut stmt = connection.prepare("SELECT hash, payload FROM transactions")?;
    let rows = stmt.query_map([], |row| {
        let hash: String = row.get(0)?;
        let payload: String = row.get(1)?;
        Ok((hash, payload))
    })?;
    for row in rows {
        let (hash, payload) = row?;
        let tx: IndexedTransaction = serde_json::from_str(&payload)?;
        transactions_by_hash.insert(hash, tx);
    }

    let mut addresses = BTreeMap::new();
    let mut stmt = connection.prepare("SELECT address, payload FROM addresses")?;
    let rows = stmt.query_map([], |row| {
        let address: String = row.get(0)?;
        let payload: String = row.get(1)?;
        Ok((address, payload))
    })?;
    for row in rows {
        let (address, payload) = row?;
        let activity: AddressActivity = serde_json::from_str(&payload)?;
        addresses.insert(address, activity);
    }

    let mut miner_rewards_by_block = BTreeMap::new();
    let mut fees_by_block = BTreeMap::new();
    let mut stmt =
        connection.prepare("SELECT height, miner_reward_atomic, fees_atomic FROM block_rewards")?;
    let rows = stmt.query_map([], |row| {
        let height: i64 = row.get(0)?;
        let reward: i64 = row.get(1)?;
        let fees: i64 = row.get(2)?;
        Ok((height as u64, reward as u64, fees as u64))
    })?;
    for row in rows {
        let (height, reward, fees) = row?;
        miner_rewards_by_block.insert(height, reward);
        fees_by_block.insert(height, fees);
    }

    Ok(IndexData {
        summary,
        blocks_by_height,
        blocks_by_hash,
        transactions_by_hash,
        addresses,
        miner_rewards_by_block,
        fees_by_block,
    })
}

/// Indexed lookup by block height or hash (SQLite primary/unique indexes).
pub fn find_block_db(index_dir: &Path, query: &str) -> IndexerResult<Option<IndexedBlock>> {
    if !index_db_path(index_dir).exists() && index_file_path(index_dir).exists() {
        // Trigger one-time migration, then query.
        let _ = load_index(index_dir)?;
    }
    if !index_db_path(index_dir).exists() {
        return Ok(None);
    }
    let connection = open_db(index_dir)?;
    if let Ok(height) = query.parse::<u64>() {
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload FROM blocks WHERE height = ?1",
                params![height as i64],
                |row| row.get(0),
            )
            .optional()?;
        return Ok(payload.map(|p| serde_json::from_str(&p)).transpose()?);
    }
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload FROM blocks WHERE hash = ?1",
            params![query],
            |row| row.get(0),
        )
        .optional()?;
    Ok(payload.map(|p| serde_json::from_str(&p)).transpose()?)
}

pub fn find_transaction_db(
    index_dir: &Path,
    hash: &str,
) -> IndexerResult<Option<IndexedTransaction>> {
    if !index_db_path(index_dir).exists() && index_file_path(index_dir).exists() {
        let _ = load_index(index_dir)?;
    }
    if !index_db_path(index_dir).exists() {
        return Ok(None);
    }
    let connection = open_db(index_dir)?;
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload FROM transactions WHERE hash = ?1",
            params![hash],
            |row| row.get(0),
        )
        .optional()?;
    Ok(payload.map(|p| serde_json::from_str(&p)).transpose()?)
}

pub fn find_address_db(index_dir: &Path, address: &str) -> IndexerResult<Option<AddressActivity>> {
    if !index_db_path(index_dir).exists() && index_file_path(index_dir).exists() {
        let _ = load_index(index_dir)?;
    }
    if !index_db_path(index_dir).exists() {
        return Ok(None);
    }
    let connection = open_db(index_dir)?;
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload FROM addresses WHERE address = ?1",
            params![address],
            |row| row.get(0),
        )
        .optional()?;
    Ok(payload.map(|p| serde_json::from_str(&p)).transpose()?)
}

pub fn reset_index_dir(index_dir: &Path) -> IndexerResult<()> {
    if index_dir.exists() {
        fs::remove_dir_all(index_dir)?;
    }
    fs::create_dir_all(index_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexSummary, SupplySummary, INDEXER_MODE};

    fn empty_index() -> IndexData {
        IndexData {
            summary: IndexSummary {
                mode: INDEXER_MODE.to_owned(),
                network: "alvenqis-devnet".into(),
                status: "test".into(),
                indexed_height: Some(0),
                indexed_block_count: 1,
                transaction_count: 1,
                address_count: 1,
                tip_hash: Some("aa".into()),
                latest_block_hash: Some("aa".into()),
                latest_block_timestamp: Some(1),
                supply: SupplySummary {
                    emitted_supply_atomic: 0,
                    max_supply_atomic: 1,
                    remaining_supply_atomic: 1,
                },
            },
            blocks_by_height: BTreeMap::new(),
            blocks_by_hash: BTreeMap::new(),
            transactions_by_hash: BTreeMap::new(),
            addresses: BTreeMap::new(),
            miner_rewards_by_block: BTreeMap::new(),
            fees_by_block: BTreeMap::new(),
        }
    }

    #[test]
    fn write_and_load_roundtrip_sqlite() {
        let dir = tempfile::tempdir().expect("temp");
        let mut index = empty_index();
        let block = IndexedBlock {
            height: 0,
            hash: "deadbeef".into(),
            previous_hash: "00".into(),
            merkle_root: "11".into(),
            timestamp: 1,
            nonce: 0,
            difficulty_leading_zero_bits: 4,
            transaction_count: 1,
            miner_address: "dalve1test".into(),
            coinbase_payout_atomic: 1,
            miner_reward_atomic: 1,
            fees_atomic: 0,
            burned_fees_atomic: 0,
            priority_fees_atomic: 0,
            base_fee_atomic: 1,
            transaction_hashes: vec!["tx1".into()],
        };
        index.blocks_by_height.insert(0, block.clone());
        index.blocks_by_hash.insert(block.hash.clone(), block);
        write_index(dir.path(), &index).expect("write");
        assert!(index_db_path(dir.path()).exists());
        let loaded = load_index(dir.path()).expect("load");
        assert_eq!(loaded.summary.tip_hash, index.summary.tip_hash);
        assert_eq!(
            loaded.blocks_by_hash.get("deadbeef").map(|b| b.height),
            Some(0)
        );
        let found = find_block_db(dir.path(), "deadbeef")
            .expect("query")
            .expect("present");
        assert_eq!(found.height, 0);
    }

    #[test]
    fn migrates_legacy_json_once_and_keeps_artifact() {
        let dir = tempfile::tempdir().expect("temp");
        let index = empty_index();
        let json_path = index_file_path(dir.path());
        fs::create_dir_all(dir.path()).expect("mkdir");
        fs::write(&json_path, serde_json::to_vec_pretty(&index).expect("json"))
            .expect("write json");
        assert!(!index_db_path(dir.path()).exists());
        let loaded = load_index(dir.path()).expect("migrate");
        assert_eq!(loaded.summary.network, "alvenqis-devnet");
        assert!(index_db_path(dir.path()).exists());
        assert!(
            json_path.exists(),
            "legacy index.json must remain as rollback artifact"
        );
    }
}
