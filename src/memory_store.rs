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

    pub fn snapshot<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.flush()?;
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

    // ... rest of impl unchanged ...
}
