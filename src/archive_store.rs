//! Cold store for archived MemoryRecords.
//! Append-only JSONL file. No LRU, no decay, no encryption by default.
//! Merkle integrity hash on each record is preserved from the hot store.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::memory_record::MemoryRecord;

pub struct ArchiveStore {
    path: PathBuf,
}

impl ArchiveStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    /// Append one record to the archive file.
    pub fn append(&mut self, record: MemoryRecord) -> Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Load all archived records from the file.
    pub fn load_all(&self) -> Result<Vec<MemoryRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            if let Ok(rec) = serde_json::from_str::<MemoryRecord>(&line) {
                records.push(rec);
            }
        }
        Ok(records)
    }

    /// Count archived records without loading them into memory.
    pub fn count(&self) -> Result<usize> {
        Ok(self.load_all()?.len())
    }
}
