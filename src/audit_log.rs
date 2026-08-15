use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub outcome: String,
    pub prev_hash: Option<String>,
    pub hash: String,
}

pub struct AuditLog {
    path: String,
    last_hash: Option<String>,
}

impl AuditLog {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let mut log = Self {
            path: path_str,
            last_hash: None,
        };
        log.load()?;
        Ok(log)
    }

    /// No-op audit log for in-memory/test use. append() is a no-op.
    pub fn new_sink() -> Self {
        Self {
            path: String::new(),
            last_hash: None,
        }
    }

    fn load(&mut self) -> anyhow::Result<()> {
        if !Path::new(&self.path).exists() {
            return Ok(());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;
            self.last_hash = Some(entry.hash);
        }
        Ok(())
    }

    pub fn append(&mut self, actor: &str, action: &str, outcome: &str) -> anyhow::Result<()> {
        if self.path.is_empty() {
            return Ok(());
        } // sink mode
        let timestamp = Utc::now();
        let prev = self.last_hash.clone();
        let mut hasher = Sha256::new();
        if let Some(ref h) = prev {
            hasher.update(h.as_bytes());
        }
        hasher.update(actor.as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(outcome.as_bytes());
        hasher.update(
            timestamp
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .to_be_bytes(),
        );
        let hash = hex::encode(hasher.finalize());
        let entry = AuditEntry {
            timestamp,
            actor: actor.to_string(),
            action: action.to_string(),
            outcome: outcome.to_string(),
            prev_hash: prev.clone(),
            hash: hash.clone(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, &entry)?;
        file.write_all(b"\n")?;
        self.last_hash = Some(hash);
        Ok(())
    }

    pub fn export(&self) -> anyhow::Result<Vec<AuditEntry>> {
        if !Path::new(&self.path).exists() {
            return Ok(vec![]);
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            entries.push(serde_json::from_str::<AuditEntry>(&line)?);
        }
        Ok(entries)
    }

    /// Return the path to the audit log file.
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn verify(&self) -> anyhow::Result<bool> {
        if !Path::new(&self.path).exists() {
            return Ok(true);
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut prev: Option<String> = None;
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;
            let mut hasher = Sha256::new();
            if let Some(ref h) = prev {
                hasher.update(h.as_bytes());
            }
            hasher.update(entry.actor.as_bytes());
            hasher.update(entry.action.as_bytes());
            hasher.update(entry.outcome.as_bytes());
            hasher.update(
                entry
                    .timestamp
                    .timestamp_nanos_opt()
                    .unwrap_or_default()
                    .to_be_bytes(),
            );
            let hash = hex::encode(hasher.finalize());
            if hash != entry.hash {
                return Ok(false);
            }
            prev = Some(hash);
        }
        Ok(true)
    }

    /// Append a continuation checkpoint as a typed audit entry.
    pub fn append_checkpoint(
        &mut self,
        actor: &str,
        checkpoint: &crate::continuation_checkpoint::ContinuationCheckpoint,
    ) -> anyhow::Result<()> {
        let serialized = serde_json::to_string(checkpoint)?;
        self.append(actor, "checkpoint", &serialized)
    }

    /// Load the latest continuation checkpoint for a task from the audit log.
    pub fn load_latest_checkpoint(
        &self,
        task_id: &uuid::Uuid,
    ) -> Option<crate::continuation_checkpoint::ContinuationCheckpoint> {
        use std::io::BufRead;
        if !Path::new(&self.path).exists() {
            return None;
        }
        let file = std::fs::File::open(&self.path).ok()?;
        let reader = std::io::BufReader::new(file);
        let mut latest: Option<crate::continuation_checkpoint::ContinuationCheckpoint> = None;
        for line in reader.lines() {
            let line = line.ok()?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.action != "checkpoint" {
                continue;
            }
            let cp: crate::continuation_checkpoint::ContinuationCheckpoint =
                match serde_json::from_str(&entry.outcome) {
                    Ok(cp) => cp,
                    Err(_) => continue,
                };
            if &cp.task_id != task_id {
                continue;
            }
            match &latest {
                Some(existing) if cp.sequence_number > existing.sequence_number => {
                    latest = Some(cp);
                }
                None => {
                    latest = Some(cp);
                }
                _ => {}
            }
        }
        latest
    }

    /// Count checkpoints for a task.
    pub fn count_checkpoints(&self, task_id: &uuid::Uuid) -> usize {
        use std::io::BufRead;
        if !Path::new(&self.path).exists() {
            return 0;
        }
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return 0,
        };
        let reader = std::io::BufReader::new(file);
        let mut count = 0;
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.action == "checkpoint" && entry.outcome.contains(&task_id.to_string()) {
                count += 1;
            }
        }
        count
    }
}
