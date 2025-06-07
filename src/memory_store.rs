use indexmap::IndexMap;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::audit_log::AuditLog;
use crate::memory_record::MemoryRecord;
use crate::persistence::{FileBackend, MemoryBackend};
use anyhow::Result;

pub struct MemoryStore<B: MemoryBackend> {
    backend: B,
    records: Vec<MemoryRecord>,
    audit: AuditLog,
    buffer: VecDeque<MemoryRecord>,
    batch_size: usize,
    index_actor: IndexMap<String, Vec<usize>>,

    index_action: IndexMap<String, Vec<usize>>,
    index_target: IndexMap<String, Vec<usize>>,

}

impl MemoryStore<FileBackend> {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new_with_options(path, 1, false)
    }

    pub fn new_with_options<P: AsRef<Path>>(path: P, batch: usize, compress: bool) -> Result<Self> {
        let backend = if compress {
            FileBackend::new_compressed(&path)?
        } else {
            FileBackend::new(&path)?
        };
        let audit_path = path.as_ref().with_extension("audit.log");
        let mut store = Self {
            backend,
            records: Vec::new(),
            audit: AuditLog::new(&audit_path)?,
            buffer: VecDeque::new(),
            batch_size: batch,
            index_actor: IndexMap::new(),

            index_action: IndexMap::new(),
            index_target: IndexMap::new(),

        };
        store.load()?;
        Ok(store)
    }

    pub fn new_encrypted<P: AsRef<Path>>(path: P, key: [u8; 32]) -> Result<Self> {
        let backend = FileBackend::new_encrypted(&path, key)?;
        let audit_path = path.as_ref().with_extension("audit.log");
        let mut store = Self {
            backend,
            records: Vec::new(),
            audit: AuditLog::new(&audit_path)?,
            buffer: VecDeque::new(),
            batch_size: 8,
            index_actor: IndexMap::new(),

            index_action: IndexMap::new(),
            index_target: IndexMap::new(),

        };
        store.load()?;
        Ok(store)
    }

    pub fn new_encrypted_envelope<P: AsRef<Path>>(path: P, master_key: [u8; 32]) -> Result<Self> {
        let backend = if path.as_ref().exists() {
            FileBackend::open_encrypted_envelope(&path, master_key)?
        } else {
            FileBackend::new_encrypted_envelope(&path, master_key)?
        };
        let audit_path = path.as_ref().with_extension("audit.log");
        let mut store = Self {
            backend,
            records: Vec::new(),
            audit: AuditLog::new(&audit_path)?,
            buffer: VecDeque::new(),
            batch_size: 8,
            index_actor: IndexMap::new(),

            index_action: IndexMap::new(),
            index_target: IndexMap::new(),

        };
        store.load()?;
        Ok(store)
    }

    fn load(&mut self) -> Result<()> {
        self.records = self.backend.load()?;
        self.index_actor.clear();

        self.index_action.clear();
        self.index_target.clear();

        for (i, rec) in self.records.iter().enumerate() {
            self.index_actor
                .entry(rec.actor.clone())
                .or_default()
                .push(i);

            self.index_action
                .entry(rec.action.clone())
                .or_default()
                .push(i);
            self.index_target
                .entry(rec.target.clone())
                .or_default()
                .push(i);

        }
        Ok(())
    }
}

impl<B: MemoryBackend> MemoryStore<B> {
    pub fn add(&mut self, record: MemoryRecord) -> Result<()> {
        self.records.push(record.clone());
        self.buffer.push_back(record.clone());
        let idx = self.records.len() - 1;
        self.index_actor
            .entry(record.actor.clone())
            .or_default()
            .push(idx);

        self.index_action
            .entry(record.action.clone())
            .or_default()
            .push(idx);
        self.index_target
            .entry(record.target.clone())
            .or_default()
            .push(idx);

        self.audit
            .append(&record.actor, &record.action, &record.target)?;
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        while let Some(rec) = self.buffer.pop_front() {
            self.backend.append(&rec)?;
        }
        self.backend.flush()?;
        Ok(())
    }

    pub fn all(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub fn find_by_actor(&self, actor: &str) -> Vec<&MemoryRecord> {
        if let Some(ids) = self.index_actor.get(actor) {
            ids.iter().filter_map(|&i| self.records.get(i)).collect()
        } else {
            Vec::new()
        }
    }


    pub fn find_by_action(&self, action: &str) -> Vec<&MemoryRecord> {
        if let Some(ids) = self.index_action.get(action) {
            ids.iter().filter_map(|&i| self.records.get(i)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_by_target(&self, target: &str) -> Vec<&MemoryRecord> {
        if let Some(ids) = self.index_target.get(target) {
            ids.iter().filter_map(|&i| self.records.get(i)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.index_actor.clear();
        self.index_action.clear();
        self.index_target.clear();
        let _ = self.backend.clear();
    }

    pub fn snapshot<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.flush()?;

        let mut file = std::fs::File::create(path)?;
        for rec in &self.records {
            serde_json::to_writer(&mut file, rec)?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn rollback<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let rec: MemoryRecord = serde_json::from_str(&line)?;
            if let Some(hash) = &rec.integrity {
                if *hash != rec.compute_hash() {
                    return Err(anyhow::anyhow!("integrity mismatch"));
                }
            }
            records.push(rec);
        }
        self.records = records.clone();
        self.index_actor.clear();

        self.index_action.clear();
        self.index_target.clear();

        for (i, rec) in self.records.iter().enumerate() {
            self.index_actor
                .entry(rec.actor.clone())
                .or_default()
                .push(i);

            self.index_action
                .entry(rec.action.clone())
                .or_default()
                .push(i);
            self.index_target
                .entry(rec.target.clone())
                .or_default()
                .push(i);

        }
        self.backend.clear()?;
        for rec in &records {
            self.backend.append(rec)?;
        }
        self.backend.flush()?;
        self.audit.append("system", "rollback", "ok")?;
        Ok(())
    }
}

impl<B: MemoryBackend> Drop for MemoryStore<B> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_read() {
        let path = "test_store.jsonl";
        let _ = std::fs::remove_file(path);
        let mut store = MemoryStore::new(path).unwrap();
        let rec = MemoryRecord::new(
            crate::memory_record::MemoryType::Symbolic,
            "a".into(),
            "b".into(),
            "c".into(),
            serde_json::json!({}),
        );
        store.add(rec).unwrap();
        assert_eq!(store.all().len(), 1);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
