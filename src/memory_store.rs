use indexmap::IndexMap;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::audit_log::AuditLog;
use crate::embedding_provider::EmbeddingProvider;
use crate::memory_record::MemoryRecord;
use crate::persistence::{FileBackend, InMemoryBackend, MemoryBackend};
use crate::source_trust::SourceTrustRegistry;
#[cfg(feature = "rocksdb-backend")]
use crate::rocksdb_backend::RocksDbBackend;
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
    /// Source trust registry for credibility-weighted memory operations.
    pub source_trust: SourceTrustRegistry,
    /// Optional embedding provider for zero-config auto-embedding on ingest.
    pub embedding_provider: Option<std::sync::Arc<dyn EmbeddingProvider>>,
    /// Active namespace for multi-tenant isolation. When set, all operations
    /// are scoped to this namespace. Records are tagged with `ns:<namespace>`.
    pub namespace: Option<String>,
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
            source_trust: SourceTrustRegistry::new(),
            embedding_provider: None,
            namespace: None,
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
            source_trust: SourceTrustRegistry::new(),
            embedding_provider: None,
            namespace: None,
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
            source_trust: SourceTrustRegistry::new(),
            embedding_provider: None,
            namespace: None,
        };
        store.load()?;
        Ok(store)
    }
}

/// Per-record error detail returned by bulk add operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BulkAddError {
    pub index: usize,
    pub actor: String,
    pub reason: String,
}

impl MemoryStore<InMemoryBackend> {
    /// Create a zero-file-I/O store for testing and ephemeral use.
    pub fn new_in_memory() -> Self {
        let backend = InMemoryBackend::new();
        Self {
            backend,
            records: Vec::new(),
            audit: AuditLog::new_sink(),
            buffer: VecDeque::new(),
            batch_size: 1,
            index_actor: IndexMap::new(),
            index_action: IndexMap::new(),
            index_target: IndexMap::new(),
            source_trust: SourceTrustRegistry::new(),
            embedding_provider: None,
            namespace: None,
        }
    }
}

#[cfg(feature = "rocksdb-backend")]
impl MemoryStore<RocksDbBackend> {
    pub fn new_rocksdb<P: AsRef<Path>>(path: P, batch: usize) -> Result<Self> {
        let backend = RocksDbBackend::new(&path)?;
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
            source_trust: SourceTrustRegistry::new(),
            embedding_provider: None,
            namespace: None,
        };
        store.load()?;
        Ok(store)
    }
}

impl<B: MemoryBackend> MemoryStore<B> {
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

    /// Rebuild all positional indices from scratch after structural Vec changes.
    /// Called after any retain/remove operation to keep index_actor / index_action /
    /// index_target consistent with the current Vec positions.
    fn rebuild_indices(&mut self) {
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
    }

    /// Remove all records whose `expires_at` is in the past.
    /// Rebuilds indices if any records were removed.
    /// Returns the number of records removed.
    pub fn purge_expired(&mut self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let before = self.records.len();
        self.records
            .retain(|r| r.expires_at.map_or(true, |exp| exp > now));
        let removed = before - self.records.len();
        if removed > 0 {
            self.rebuild_indices();
        }
        removed
    }

    /// Remove the single record with the given `id`.
    /// Rebuilds indices if a record was deleted.
    /// Returns `true` if a record was found and removed, `false` if not found.
    pub fn delete_by_id(&mut self, id: uuid::Uuid) -> bool {
        let before = self.records.len();
        self.records.retain(|r| r.id != id);
        let removed = before - self.records.len();
        if removed > 0 {
            self.rebuild_indices();
            true
        } else {
            false
        }
    }

    pub fn add(&mut self, mut record: MemoryRecord) -> Result<()> {
        // Auto-tag with namespace for multi-tenant isolation
        if let Some(ref ns) = self.namespace {
            let ns_tag = format!("ns:{}", ns);
            if !record.tags.contains(&ns_tag) {
                record.tags.push(ns_tag);
            }
        }
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

    pub fn flush(&mut self) -> Result<()> {
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

    /// Find records that contain ANY of the given tags.
    pub fn find_by_tags(&self, tags: &[&str]) -> Vec<&MemoryRecord> {
        if tags.is_empty() {
            return Vec::new();
        }
        self.records.iter()
            .filter(|r| tags.iter().any(|t| r.tags.contains(&t.to_string())))
            .collect()
    }

    /// Find records matching any actor in `actors`. Returns empty vec for empty slice.
    pub fn find_by_actors(&self, actors: &[&str]) -> Vec<&MemoryRecord> {
        if actors.is_empty() { return Vec::new(); }
        self.records.iter()
            .filter(|r| actors.contains(&r.actor.as_str()))
            .collect()
    }

    /// Set `status` on a record by UUID. Returns error if not found.
    pub fn set_status(&mut self, id: uuid::Uuid, status: &str) -> Result<()> {
        let idx = self.records.iter().position(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("record not found: {}", id))?;
        self.records[idx].status = status.to_string();
        self.records[idx].integrity = Some(self.records[idx].compute_hash());
        // Rewrite backend
        self.backend.clear()?;
        let snap = self.records.clone();
        for rec in &snap { self.backend.append(rec)?; }
        self.backend.flush()?;
        Ok(())
    }

    /// Boost confidence by 0.10 (clamped to 1.0). Returns (before, after).
    /// Also records corroboration in the source trust registry for this record's source.
    pub fn corroborate(&mut self, id: uuid::Uuid) -> Result<(f32, f32)> {
        let idx = self.records.iter().position(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("record not found: {}", id))?;
        let before = self.records[idx].confidence;
        let after = (before + 0.10).min(1.0);
        self.records[idx].confidence = after;
        self.records[idx].integrity = Some(self.records[idx].compute_hash());
        // Track source trust
        if let Some(ref source) = self.records[idx].source {
            self.source_trust.record_corroboration(source);
        }
        self.backend.clear()?;
        let snap = self.records.clone();
        for rec in &snap { self.backend.append(rec)?; }
        self.backend.flush()?;
        Ok((before, after))
    }

    /// Reduce confidence by 0.15 (clamped to 0.0). Auto-quarantines if result < 0.30.
    /// Also records contradiction in the source trust registry for this record's source.
    /// Returns (before, after, was_quarantined).
    pub fn contradict(&mut self, id: uuid::Uuid) -> Result<(f32, f32, bool)> {
        let idx = self.records.iter().position(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("record not found: {}", id))?;
        let before = self.records[idx].confidence;
        let after = (before - 0.15).max(0.0);
        self.records[idx].confidence = after;
        let quarantined = after < 0.30;
        if quarantined { self.records[idx].status = "quarantine".to_string(); }
        self.records[idx].integrity = Some(self.records[idx].compute_hash());
        // Track source trust
        if let Some(ref source) = self.records[idx].source {
            self.source_trust.record_contradiction(source);
        }
        self.backend.clear()?;
        let snap = self.records.clone();
        for rec in &snap { self.backend.append(rec)?; }
        self.backend.flush()?;
        Ok((before, after, quarantined))
    }

    /// Semantic search: rank records by cosine similarity against `query_embedding`
    /// if they carry a `metadata.embedding` float array, otherwise fall back to
    /// keyword matching against actor + action + target.
    /// Quarantined records are excluded unless `include_quarantined` is true.
    /// Results are trust-weighted: score is multiplied by source credibility.
    /// Returns up to `limit` records sorted by descending score.
    pub fn search_semantic(
        &self,
        query_embedding: Option<&[f64]>,
        query_text: &str,
        limit: usize,
        include_quarantined: bool,
    ) -> Vec<(&MemoryRecord, f64)> {
        let now_ts = chrono::Utc::now().timestamp();
        let mut scored: Vec<(&MemoryRecord, f64)> = self
            .records
            .iter()
            .filter(|r| {
                (include_quarantined || r.status != "quarantine")
                    && r.expires_at.map_or(true, |exp| exp > now_ts)
            })
            .map(|rec| {
                let base_score = if let Some(qe) = query_embedding {
                    let doc_vec: Option<Vec<f64>> = rec
                        .metadata
                        .get("embedding")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    if let Some(dv) = doc_vec {
                        cosine_similarity(qe, &dv)
                    } else {
                        keyword_score(query_text, rec)
                    }
                } else {
                    keyword_score(query_text, rec)
                };
                // Trust-weighted score: multiply by source credibility
                let trust = rec.source.as_deref()
                    .map(|s| self.source_trust.get_trust(s) as f64)
                    .unwrap_or(0.5);
                // Priority multiplier: high=1.5×, low=0.5×, normal/pinned=1.0×
                // Note: pinned records take a separate code path (score 2.0 override).
                let priority_mult: f64 = match rec.priority.as_str() {
                    "high" => 1.5,
                    "low"  => 0.5,
                    _      => 1.0,
                };
                let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult;
                (rec, weighted)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Pinned active records always appear first, regardless of score
        let mut pinned: Vec<(&MemoryRecord, f64)> = self.records.iter()
            .filter(|r| {
                r.priority == "pinned"
                    && (include_quarantined || r.status != "quarantine")
                    && r.expires_at.map_or(true, |exp| exp > now_ts)
            })
            .map(|r| (r, 2.0f64))
            .collect();
        let scored_ids: std::collections::HashSet<uuid::Uuid> = scored.iter().map(|(r, _)| r.id).collect();
        pinned.retain(|(r, _)| !scored_ids.contains(&r.id));
        pinned.extend(scored);
        pinned.truncate(limit);
        pinned
    }

    /// Find records in the active namespace only.
    /// Filters to records tagged with `ns:<namespace>` if namespace is set.
    pub fn in_namespace(&self) -> Vec<&MemoryRecord> {
        match self.namespace {
            Some(ref ns) => {
                let tag = format!("ns:{}", ns);
                self.records.iter().filter(|r| r.tags.contains(&tag)).collect()
            }
            None => self.records.iter().collect(),
        }
    }

    /// Find records by actor within the active namespace.
    pub fn find_by_actor_ns(&self, actor: &str) -> Vec<&MemoryRecord> {
        let candidates = self.find_by_actor(actor);
        match self.namespace {
            Some(ref ns) => {
                let tag = format!("ns:{}", ns);
                candidates.into_iter().filter(|r| r.tags.contains(&tag)).collect()
            }
            None => candidates,
        }
    }

    /// List all unique namespaces from record tags.
    pub fn list_namespaces(&self) -> Vec<String> {
        let mut namespaces: Vec<String> = self.records.iter()
            .filter_map(|r| r.tags.iter().find(|t| t.starts_with("ns:")))
            .map(|t| t[3..].to_string())
            .collect();
        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    /// Count records per namespace.
    pub fn namespace_counts(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for rec in &self.records {
            for tag in &rec.tags {
                if let Some(ns) = tag.strip_prefix("ns:") {
                    *counts.entry(ns.to_string()).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    /// Find records from trusted sources only.
    /// Filters to records whose source meets or exceeds `trust_threshold`.
    pub fn find_trusted(&self, actor: &str, trust_threshold: f64) -> Vec<&MemoryRecord> {
        self.find_by_actor(actor)
            .into_iter()
            .filter(|r| {
                r.source.as_deref()
                    .map(|s| self.source_trust.is_trusted(s, trust_threshold))
                    .unwrap_or(false) // records without source are not trusted
            })
            .collect()
    }

    /// Set the active namespace for multi-tenant isolation.
    /// When set, add() auto-tags records with `ns:<namespace>` and queries
    /// are scoped to this namespace.
    pub fn with_namespace(mut self, ns: String) -> Self {
        self.namespace = Some(ns);
        self
    }

    /// Set an embedding provider for zero-config auto-embedding.
    pub fn with_embedding_provider(mut self, provider: std::sync::Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// Add a record and auto-embed its text content if an embedding provider is set.
    /// The embedding is stored in `metadata.embedding` for later semantic search.
    pub fn embed_and_add(&mut self, record: MemoryRecord, text_to_embed: &str) -> Result<()> {
        if let Some(ref provider) = self.embedding_provider {
            let embedding: Vec<f64> = provider.embed(text_to_embed)
                .into_iter()
                .map(|v| v as f64)
                .collect();
            let mut rec = record;
            rec.set_metadata_field("embedding", embedding).ok();
            self.add(rec)
        } else {
            self.add(record)
        }
    }

    /// GDPR right-to-forget: remove all records for `actor`, rewrite the backend
    /// file without them, and append a deletion entry to the audit log.
    /// Returns the UUIDs of deleted records.
    pub fn delete_by_actor(&mut self, actor: &str) -> Result<Vec<uuid::Uuid>> {
        let deleted_ids: Vec<uuid::Uuid> = self.records.iter()
            .filter(|r| r.actor == actor)
            .map(|r| r.id)
            .collect();

        if deleted_ids.is_empty() {
            return Ok(vec![]);
        }

        // Remove from in-memory records and pending write buffer
        self.records.retain(|r| r.actor != actor);
        self.buffer.retain(|r| r.actor != actor);

        // Rebuild index maps from scratch
        self.index_actor.clear();
        self.index_action.clear();
        self.index_target.clear();
        for (i, rec) in self.records.iter().enumerate() {
            self.index_actor.entry(rec.actor.clone()).or_default().push(i);
            self.index_action.entry(rec.action.clone()).or_default().push(i);
            self.index_target.entry(rec.target.clone()).or_default().push(i);
        }

        // Rewrite backend: clear the file then re-append all remaining records
        self.backend.clear()?;
        let remaining = self.records.clone();
        for record in &remaining {
            self.backend.append(record)?;
        }
        self.backend.flush()?;

        // Record the deletion in the tamper-evident audit log
        self.audit.append(
            actor,
            "gdpr_forget",
            &format!("purged {} records", deleted_ids.len()),
        )?;

        Ok(deleted_ids)
    }

    /// Find a single record by its UUID.
    pub fn find_by_id(&self, id: uuid::Uuid) -> Option<&MemoryRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// Update a record in-place: apply partial changes, increment version,
    /// rewrite the backend file, and append an update entry to the audit log.
    pub fn update_record(
        &mut self,
        id: uuid::Uuid,
        new_target: Option<&str>,
        new_action: Option<&str>,
        new_confidence: Option<f32>,
        new_source: Option<&str>,
        new_metadata: Option<serde_json::Value>,
    ) -> Result<uuid::Uuid> {
        let idx = self.records.iter().position(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("record not found: {}", id))?;

        // Apply partial updates
        if let Some(t) = new_target  { self.records[idx].target = t.to_string(); }
        if let Some(a) = new_action  { self.records[idx].action = a.to_string(); }
        if let Some(c) = new_confidence { self.records[idx].confidence = c.clamp(0.0, 1.0); }
        if let Some(s) = new_source  { self.records[idx].source = Some(s.to_string()); }
        if let Some(m) = new_metadata { self.records[idx].metadata = m; }
        self.records[idx].version += 1;
        // Recompute integrity hash after update
        self.records[idx].integrity = Some(self.records[idx].compute_hash());

        // Rewrite backend
        self.backend.clear()?;
        let records_clone = self.records.clone();
        for record in &records_clone {
            self.backend.append(record)?;
        }
        self.backend.flush()?;

        // Audit log
        let actor = self.records[idx].actor.clone();
        let version = self.records[idx].version;
        self.audit.append(
            &actor,
            "update",
            &format!("record {} updated to version {}", id, version),
        )?;

        Ok(id)
    }

    /// Return the most recent record(s) per (actor, action) combination.
    /// If action is None, returns most recent record per actor.
    /// Useful for "what is the current value of X?" queries.
    pub fn find_latest(
        &self,
        actor: Option<&str>,
        action: Option<&str>,
        limit: usize,
    ) -> Vec<&MemoryRecord> {
        let now_ts = chrono::Utc::now().timestamp();
        // Collect non-expired candidates
        let mut candidates: Vec<&MemoryRecord> = self.records.iter()
            .filter(|r| {
                actor.map_or(true, |a| r.actor == a) &&
                action.map_or(true, |ac| r.action == ac) &&
                r.expires_at.map_or(true, |exp| exp > now_ts)
            })
            .collect();

        // Sort descending by timestamp (newest first)
        candidates.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Deduplicate: keep only the most recent per (actor, action) pair
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        let deduplicated: Vec<&MemoryRecord> = candidates.into_iter()
            .filter(|r| seen.insert((r.actor.clone(), r.action.clone())))
            .collect();

        deduplicated.into_iter().take(limit).collect()
    }

    /// Verify audit log Merkle chain integrity. Returns (intact, entry_count).
    pub fn audit_verify(&self) -> anyhow::Result<(bool, usize)> {
        let entries = self.audit.export()?;
        let count = entries.len();
        let intact = self.audit.verify()?;
        Ok((intact, count))
    }

    /// Export all audit log entries.
    pub fn audit_export(&self) -> anyhow::Result<Vec<crate::audit_log::AuditEntry>> {
        self.audit.export()
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

// ---------------------------------------------------------------------------
// Free functions used by MemoryStore::search_semantic
// ---------------------------------------------------------------------------

/// Cosine similarity between two f64 slices. Returns 0.0 if either is zero-length.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let dot: f64    = a[..len].iter().zip(b[..len].iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64  = a[..len].iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64  = b[..len].iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }
}

/// Simple keyword score: fraction of whitespace-split query tokens found in the
/// concatenated `actor + action + target` string (case-insensitive).
pub fn keyword_score(query: &str, rec: &crate::memory_record::MemoryRecord) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let haystack = format!("{} {} {}", rec.actor, rec.action, rec.target).to_lowercase();
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let hits = tokens.iter().filter(|t| haystack.contains(&t.to_lowercase())).count();
    hits as f64 / tokens.len() as f64
}
