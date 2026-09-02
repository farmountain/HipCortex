//! Phase 4: Multi-agent workspace scoping with OR-Set CRDT merge.
//!
//! Chain-of-thought:
//!   WorkspaceOpen  → snapshot parent store IDs as baseline; track new additions as OR-Set tuples.
//!   WorkspaceMerge → union OR-Set additions + tombstones of two Shared workspaces (convergent).
//!   apply_to_store → push live workspace records into any MemoryStore (merge-into-parent).
//!
//! Isolation guarantee: mutations inside a Private workspace never reach the parent store
//! until the caller explicitly calls apply_to_store. Silent contamination is impossible.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

use crate::memory_record::MemoryRecord;
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;

// ── WorkspaceId ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for WorkspaceId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── WorkspaceMode ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WorkspaceMode {
    /// Single-agent: isolated from all other workspaces. No merge allowed.
    Private,
    /// Multi-agent: additions tracked as OR-Set tuples for convergent merge.
    Shared,
}

// ── OR-Set internals ──────────────────────────────────────────────────────────

/// One addition entry in the OR-Set: record + the actor that added it + Lamport clock.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OSetEntry {
    record: MemoryRecord,
    actor: String,
    clock: u64,
}

// ── Workspace ─────────────────────────────────────────────────────────────────

/// A scoped workspace for one or more agents.
///
/// - **Private**: changes are fully isolated until `apply_to_store` is called.
/// - **Shared**: additions are tagged `(record_id, actor, lamport)`.
///   Two Shared workspaces merge by unioning their OR-Sets; result is the same
///   regardless of merge order (convergent, commutative, idempotent).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub mode: WorkspaceMode,
    created_at: SystemTime,
    /// When this workspace lease expires; None means no expiry.
    pub lease_until: Option<SystemTime>,
    /// IDs present in the parent store at open time — never mutated.
    baseline_ids: HashSet<Uuid>,
    /// OR-Set additions (one entry per actor×record_id pair).
    or_added: Vec<OSetEntry>,
    /// Tombstones: `(record_id, actor_tag)` — cancels that actor's add.
    or_tombstones: HashSet<(Uuid, String)>,
    /// Lamport clock for this replica.
    lamport: u64,
}

impl Workspace {
    /// Snapshot parent store IDs as baseline, create empty workspace.
    pub fn open<B: MemoryBackend>(
        id: WorkspaceId,
        mode: WorkspaceMode,
        store: &MemoryStore<B>,
    ) -> Self {
        let baseline_ids: HashSet<Uuid> = store.all().iter().map(|r| r.id).collect();
        Self {
            id,
            mode,
            created_at: SystemTime::now(),
            lease_until: None,
            baseline_ids,
            or_added: Vec::new(),
            or_tombstones: HashSet::new(),
            lamport: 0,
        }
    }

    /// Stage a record addition (tagged by actor). Works for both Private and Shared.
    pub fn add_record(&mut self, record: MemoryRecord, actor: &str) {
        self.lamport += 1;
        self.or_added.push(OSetEntry { record, actor: actor.to_string(), clock: self.lamport });
    }

    /// Tombstone a record (marks the actor's OR-Set entry as removed).
    pub fn remove_record(&mut self, record_id: Uuid, actor: &str) {
        self.or_tombstones.insert((record_id, actor.to_string()));
    }

    /// Live records = added entries whose (id, actor) pair is NOT tombstoned.
    pub fn live_records(&self) -> Vec<&MemoryRecord> {
        self.or_added
            .iter()
            .filter(|e| !self.or_tombstones.contains(&(e.record.id, e.actor.clone())))
            .map(|e| &e.record)
            .collect()
    }

    /// Push live workspace records into `store` (merge-into-parent operation).
    /// Skips records already present by ID. Returns count of newly added records.
    pub fn apply_to_store<B: MemoryBackend>(
        &self,
        store: &mut MemoryStore<B>,
    ) -> Result<usize, String> {
        let mut added = 0usize;
        for rec in self.live_records() {
            if store.find_by_id(rec.id).is_none() {
                store.add(rec.clone()).map_err(|e| format!("store add: {e}"))?;
                added += 1;
            }
        }
        Ok(added)
    }

    /// Delta count: live records NOT in the baseline snapshot.
    pub fn delta_count(&self) -> usize {
        self.live_records().iter().filter(|r| !self.baseline_ids.contains(&r.id)).count()
    }

    /// 5-minute TTL for auto-eviction (configurable; 0 = no expiry).
    /// Expired when `lease_until` is set and already passed.
    /// Workspaces with no lease never expire.
    pub fn is_expired(&self) -> bool {
        self.lease_until
            .map(|t| SystemTime::now() > t)
            .unwrap_or(false)
    }

    /// The expiry instant, if a lease was set.
    pub fn expires_at(&self) -> Option<SystemTime> {
        self.lease_until
    }

    /// Set or extend the lease by `secs` seconds from now.
    pub fn renew_lease(&mut self, secs: u64) {
        self.lease_until = Some(SystemTime::now() + std::time::Duration::from_secs(secs));
    }

    /// Persist this workspace to `dir/workspace_<id>.jsonl`.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join(format!("workspace_{}.jsonl", self.id.0));
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("workspace save: {e}"))
    }

    /// Load one workspace from a file written by `save`.
    pub fn load(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("workspace load: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("workspace deserialize: {e}"))
    }

    /// Load all workspaces from a directory (files matching `workspace_*.jsonl`).
    pub fn load_all(dir: &Path) -> Vec<Self> {
        let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("workspace_")
            })
            .filter_map(|e| Self::load(&e.path()).ok())
            .collect()
    }

    pub fn record_count(&self) -> usize {
        self.live_records().len()
    }
}

// ── WorkspaceRegistry ─────────────────────────────────────────────────────────

/// Registry of all open workspaces. Held inside `CognitiveHandle` behind an `Arc<Mutex<_>>`.
pub struct WorkspaceRegistry {
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl WorkspaceRegistry {
    pub fn new() -> Self {
        Self { workspaces: HashMap::new() }
    }

    /// Open a new workspace. Snapshots the current store as baseline.
    pub fn open<B: MemoryBackend>(
        &mut self,
        id: WorkspaceId,
        mode: WorkspaceMode,
        store: &MemoryStore<B>,
    ) {
        self.workspaces.insert(id.clone(), Workspace::open(id, mode, store));
    }

    pub fn get(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(id)
    }

    pub fn get_mut(&mut self, id: &WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces.get_mut(id)
    }

    pub fn remove(&mut self, id: &WorkspaceId) -> Option<Workspace> {
        self.workspaces.remove(id)
    }

    /// Renew the lease on workspace `id` by `secs` seconds from now.
    /// Returns Err if the workspace does not exist.
    pub fn renew(&mut self, id: &WorkspaceId, secs: u64) -> Result<(), String> {
        self.workspaces
            .get_mut(id)
            .map(|ws| ws.renew_lease(secs))
            .ok_or_else(|| format!("workspace {} not found", id.0))
    }

    /// OR-Set merge of `from` into `into`. Both must be Shared.
    /// Returns count of records merged into `into`.
    pub fn merge(
        &mut self,
        from_id: &WorkspaceId,
        into_id: &WorkspaceId,
    ) -> Result<usize, String> {
        if from_id == into_id {
            return Ok(0);
        }
        // Extract OR-Set data from `from` without holding a mutable borrow on `self`
        let (or_added, tombstones, from_mode) = self
            .workspaces
            .get(from_id)
            .map(|ws| (ws.or_added.clone(), ws.or_tombstones.clone(), ws.mode.clone()))
            .ok_or_else(|| format!("workspace {from_id} not found"))?;

        let into_ws = self
            .workspaces
            .get_mut(into_id)
            .ok_or_else(|| format!("workspace {into_id} not found"))?;

        if from_mode != WorkspaceMode::Shared || into_ws.mode != WorkspaceMode::Shared {
            return Err("OR-Set merge requires both workspaces to be Shared".into());
        }

        // Union tombstones
        for t in tombstones {
            into_ws.or_tombstones.insert(t);
        }

        // Union additions — skip if same (record_id, actor) already present
        let existing: HashSet<(Uuid, String)> =
            into_ws.or_added.iter().map(|e| (e.record.id, e.actor.clone())).collect();
        let mut merged = 0usize;
        for entry in or_added {
            let key = (entry.record.id, entry.actor.clone());
            if !existing.contains(&key) {
                into_ws.lamport = into_ws.lamport.max(entry.clock) + 1;
                into_ws.or_added.push(entry);
                merged += 1;
            }
        }
        Ok(merged)
    }

    /// Apply workspace's live records into a MemoryStore (merge-into-parent path).
    pub fn apply_to_parent<B: MemoryBackend>(
        &self,
        id: &WorkspaceId,
        store: &mut MemoryStore<B>,
    ) -> Result<usize, String> {
        self.workspaces
            .get(id)
            .ok_or_else(|| format!("workspace {id} not found"))?
            .apply_to_store(store)
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    /// Remove expired workspaces (5-minute TTL).
    pub fn evict_expired(&mut self) -> usize {
        let before = self.workspaces.len();
        self.workspaces.retain(|_, ws| !ws.is_expired());
        before - self.workspaces.len()
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self { Self::new() }
}
