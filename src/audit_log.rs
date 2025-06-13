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
}
