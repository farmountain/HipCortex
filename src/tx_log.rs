use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxKind {
    MemoryAdd,
    MemoryUpdate,
    MemoryArchive,
    MemoryDelete,
    BeliefAssert,
    BeliefRetract,
    WorldModelObserve,
    WorldModelUpdate,
    GoalCreate,
    GoalStatusChange,
    Consolidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxEntry {
    pub tx_id: u64,
    pub timestamp_ms: u64,
    pub kind: TxKind,
    pub record_ids: Vec<Uuid>,
    pub actor: String,
}

pub struct TxLog {
    counter: Arc<AtomicU64>,
    path: PathBuf,
}

impl TxLog {
    /// Open or create log file. Counter restores from last JSONL line on startup.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let last_tx = if path.exists() {
            let file = std::fs::File::open(&path).map_err(|e| format!("TxLog::open: {e}"))?;
            let mut max_id = 0u64;
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                if let Ok(entry) = serde_json::from_str::<TxEntry>(&line) {
                    if entry.tx_id > max_id {
                        max_id = entry.tx_id;
                    }
                }
            }
            max_id
        } else {
            0
        };
        Ok(Self {
            counter: Arc::new(AtomicU64::new(last_tx + 1)),
            path,
        })
    }

    /// Append one TxEntry. Returns assigned tx_id. Infallible from caller — write errors go to stderr.
    pub fn append(&self, kind: TxKind, record_ids: Vec<Uuid>, actor: &str) -> u64 {
        let tx_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let entry = TxEntry {
            tx_id,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            kind,
            record_ids,
            actor: actor.to_string(),
        };
        match serde_json::to_string(&entry) {
            Ok(line) => {
                if let Ok(mut f) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)
                {
                    let _ = writeln!(f, "{line}");
                } else {
                    eprintln!("TxLog write error: cannot open {:?}", self.path);
                }
            }
            Err(e) => eprintln!("TxLog serialize error: {e}"),
        }
        tx_id
    }

    /// Return all entries in inclusive range [from_tx, to_tx].
    pub fn query_range(&self, from_tx: u64, to_tx: u64) -> Result<Vec<TxEntry>, String> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let file =
            std::fs::File::open(&self.path).map_err(|e| format!("TxLog::query_range: {e}"))?;
        let mut result = Vec::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<TxEntry>(&line) {
                if entry.tx_id >= from_tx && entry.tx_id <= to_tx {
                    result.push(entry);
                }
            }
        }
        Ok(result)
    }

    /// Last assigned tx_id (0 if nothing appended yet).
    pub fn current_tx(&self) -> u64 {
        self.counter.load(Ordering::SeqCst).saturating_sub(1)
    }
}
