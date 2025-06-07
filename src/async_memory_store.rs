#[cfg(feature = "async-store")]
use crate::audit_log::AuditLog;
#[cfg(feature = "async-store")]
use crate::memory_record::MemoryRecord;
#[cfg(feature = "async-store")]
use crate::persistence::AsyncMemoryBackend;
#[cfg(feature = "async-store")]
use anyhow::Result;
#[cfg(feature = "async-store")]
use std::collections::VecDeque;
#[cfg(feature = "async-store")]
use std::path::Path;

#[cfg(feature = "async-store")]
pub struct AsyncMemoryStore<B: AsyncMemoryBackend> {
    backend: B,
    records: Vec<MemoryRecord>,
    audit: AuditLog,
    buffer: VecDeque<MemoryRecord>,
    batch_size: usize,
}

#[cfg(feature = "async-store")]
impl<B: AsyncMemoryBackend> AsyncMemoryStore<B> {
    pub async fn new(mut backend: B, audit_path: &Path, batch_size: usize) -> Result<Self> {
        let mut store = Self {
            backend,
            records: Vec::new(),
            audit: AuditLog::new(audit_path)?,
            buffer: VecDeque::new(),
            batch_size,
        };
        store.load().await?;
        Ok(store)
    }

    async fn load(&mut self) -> Result<()> {
        self.records = self.backend.load().await?;
        Ok(())
    }

    pub fn all(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub async fn add(&mut self, record: MemoryRecord) -> Result<()> {
        self.records.push(record.clone());
        self.buffer.push_back(record.clone());
        self.audit
            .append(&record.actor, &record.action, &record.target)?;
        if self.buffer.len() >= self.batch_size {
            self.flush().await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        while let Some(rec) = self.buffer.pop_front() {
            self.backend.append(&rec).await?;
        }
        self.backend.flush().await?;
        Ok(())
    }
}
