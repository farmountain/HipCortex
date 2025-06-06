use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::memory_record::MemoryRecord;

pub struct MemoryStore {
    path: PathBuf,
    records: Vec<MemoryRecord>,
}

impl MemoryStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let mut records = Vec::new();
        if path_buf.exists() {
            let file = File::open(&path_buf)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let rec: MemoryRecord = serde_json::from_str(&line)?;
                records.push(rec);
            }
        }
        Ok(Self { path: path_buf, records })
    }

    pub fn add(&mut self, record: MemoryRecord) -> Result<()> {
        self.records.push(record.clone());
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        serde_json::to_writer(&mut file, &record)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub fn all(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub fn clear(&mut self) {
        self.records.clear();
        let _ = std::fs::remove_file(&self.path);
    }
}
