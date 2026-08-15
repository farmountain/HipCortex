// Lease Manager — Concurrency control for multi-agent access to the Symbolic Store
//
// Issues time-bounded, renewable leases on regions of the Symbolic Store graph.
// A session requesting to work on subgraph region R gets a lease; overlapping
// requests are denied or queued. Leases survive client death — an expired lease
// frees its region automatically, making the system self-healing.
//
// Design: Leases are authoritative, not advisory. SymbolicStore operations
// (add_node, add_edge, remove_node, update_property) check lease validity
// before allowing mutation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// A time-bounded lease on a region of the Symbolic Store graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    /// Unique lease identifier
    pub lease_id: Uuid,
    /// The session that holds this lease
    pub session_id: Uuid,
    /// Node IDs covered by this lease (the "region")
    pub region: HashSet<Uuid>,
    /// When the lease was granted
    pub granted_at: SystemTime,
    /// When the lease expires (unless renewed)
    pub expires_at: SystemTime,
    /// When the lease was last renewed
    pub renewed_at: Option<SystemTime>,
    /// Whether this lease allows write access (vs read-only)
    pub write_access: bool,
    /// Human-readable purpose for audit
    pub purpose: String,
    /// Lease metadata
    pub metadata: HashMap<String, String>,
}

/// Result of a lease acquisition attempt.
#[derive(Debug, Clone)]
pub enum LeaseResult {
    /// Lease granted successfully
    Granted(Lease),
    /// Lease denied because region overlaps with an active lease
    Conflict {
        /// The conflicting lease
        conflicting_lease: Lease,
        /// Which nodes overlap
        overlapping_nodes: Vec<Uuid>,
    },
    /// Lease denied because the region is invalid (e.g., empty)
    InvalidRegion(String),
}

/// Manager for all active leases.
pub struct LeaseManager {
    /// All active leases, keyed by lease_id
    leases: HashMap<Uuid, Lease>,
    /// Map from node_id to the set of lease_ids covering it
    node_leases: HashMap<Uuid, HashSet<Uuid>>,
    /// Default lease TTL (time-to-live)
    default_ttl: Duration,
    /// Maximum number of concurrent leases per session
    max_leases_per_session: usize,
}

impl LeaseManager {
    /// Create a new lease manager.
    pub fn new(default_ttl_secs: u64) -> Self {
        Self {
            leases: HashMap::new(),
            node_leases: HashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_secs),
            max_leases_per_session: 10,
        }
    }

    /// Create with custom settings.
    pub fn with_config(default_ttl_secs: u64, max_leases_per_session: usize) -> Self {
        Self {
            leases: HashMap::new(),
            node_leases: HashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_secs),
            max_leases_per_session,
        }
    }

    /// Attempt to acquire a lease on a graph region.
    ///
    /// Returns Granted if the region is free, Conflict if it overlaps with
    /// an active lease, or InvalidRegion if the region is empty.
    pub fn acquire(
        &mut self,
        session_id: Uuid,
        region: HashSet<Uuid>,
        write_access: bool,
        purpose: String,
    ) -> LeaseResult {
        // Validate region
        if region.is_empty() {
            return LeaseResult::InvalidRegion("Cannot lease an empty region".to_string());
        }

        // Check per-session lease limit
        let session_lease_count = self
            .leases
            .values()
            .filter(|l| l.session_id == session_id && !self.is_expired(l))
            .count();
        if session_lease_count >= self.max_leases_per_session {
            return LeaseResult::InvalidRegion(format!(
                "Session has {} active leases (max {})",
                session_lease_count, self.max_leases_per_session
            ));
        }

        // Check for conflicts with active leases
        self.cleanup_expired();

        let mut overlapping_nodes = Vec::new();
        let mut conflicting_lease: Option<Lease> = None;

        for node_id in &region {
            if let Some(lease_ids) = self.node_leases.get(node_id) {
                for lease_id in lease_ids {
                    if let Some(existing) = self.leases.get(lease_id) {
                        if !self.is_expired(existing) {
                            overlapping_nodes.push(*node_id);
                            conflicting_lease = Some(existing.clone());
                            break;
                        }
                    }
                }
                if conflicting_lease.is_some() {
                    break;
                }
            }
        }

        if let Some(conflict) = conflicting_lease {
            return LeaseResult::Conflict {
                conflicting_lease: conflict,
                overlapping_nodes,
            };
        }

        // Grant the lease
        let now = SystemTime::now();
        let lease = Lease {
            lease_id: Uuid::new_v4(),
            session_id,
            region: region.clone(),
            granted_at: now,
            expires_at: now + self.default_ttl,
            renewed_at: None,
            write_access,
            purpose,
            metadata: HashMap::new(),
        };

        let lease_id = lease.lease_id;

        // Register in node_leases index
        for node_id in &region {
            self.node_leases
                .entry(*node_id)
                .or_insert_with(HashSet::new)
                .insert(lease_id);
        }

        self.leases.insert(lease_id, lease.clone());
        LeaseResult::Granted(lease)
    }

    /// Release a lease, freeing its region.
    ///
    /// Returns true if the lease existed and was released.
    pub fn release(&mut self, lease_id: Uuid) -> bool {
        if let Some(lease) = self.leases.remove(&lease_id) {
            // Remove from node_leases index
            for node_id in &lease.region {
                if let Some(lease_ids) = self.node_leases.get_mut(node_id) {
                    lease_ids.remove(&lease_id);
                    if lease_ids.is_empty() {
                        self.node_leases.remove(node_id);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Renew a lease, extending its expiry.
    ///
    /// Returns the new expiry time, or None if the lease doesn't exist or has expired.
    pub fn renew(&mut self, lease_id: Uuid, extension: Option<Duration>) -> Option<SystemTime> {
        let ttl = extension.unwrap_or(self.default_ttl);
        // Check expiry before mutable borrow to avoid borrow conflict
        let expired = self
            .leases
            .get(&lease_id)
            .map_or(true, |l| self.is_expired(l));
        if expired {
            return None;
        }
        if let Some(lease) = self.leases.get_mut(&lease_id) {
            let now = SystemTime::now();
            lease.expires_at = now + ttl;
            lease.renewed_at = Some(now);
            Some(lease.expires_at)
        } else {
            None
        }
    }

    /// Check if a specific node is covered by any active lease.
    pub fn is_node_leased(&self, node_id: Uuid) -> bool {
        if let Some(lease_ids) = self.node_leases.get(&node_id) {
            lease_ids
                .iter()
                .any(|lid| self.leases.get(lid).map_or(false, |l| !self.is_expired(l)))
        } else {
            false
        }
    }

    /// Check if a session holds a lease on a specific node.
    pub fn session_has_lease(&self, session_id: Uuid, node_id: Uuid) -> bool {
        if let Some(lease_ids) = self.node_leases.get(&node_id) {
            lease_ids.iter().any(|lid| {
                self.leases
                    .get(lid)
                    .map_or(false, |l| l.session_id == session_id && !self.is_expired(l))
            })
        } else {
            false
        }
    }

    /// Get all active leases for a session.
    pub fn session_leases(&self, session_id: Uuid) -> Vec<&Lease> {
        self.leases
            .values()
            .filter(|l| l.session_id == session_id && !self.is_expired(l))
            .collect()
    }

    /// Get the number of active (non-expired) leases.
    pub fn active_lease_count(&self) -> usize {
        self.leases.values().filter(|l| !self.is_expired(l)).count()
    }

    /// Remove all expired leases.
    ///
    /// Returns the number of leases cleaned up.
    pub fn cleanup_expired(&mut self) -> usize {
        let expired_ids: Vec<Uuid> = self
            .leases
            .values()
            .filter(|l| self.is_expired(l))
            .map(|l| l.lease_id)
            .collect();

        for lease_id in &expired_ids {
            self.release(*lease_id);
        }

        expired_ids.len()
    }

    /// Check if a lease has expired.
    fn is_expired(&self, lease: &Lease) -> bool {
        SystemTime::now() > lease.expires_at
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new(300) // 5-minute default TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_lease() {
        let mut manager = LeaseManager::new(60);
        let session_id = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![Uuid::new_v4(), Uuid::new_v4()].into_iter().collect();

        let result = manager.acquire(session_id, region.clone(), true, "Testing".to_string());

        match result {
            LeaseResult::Granted(lease) => {
                assert_eq!(lease.session_id, session_id);
                assert_eq!(lease.region, region);
                assert!(lease.write_access);
                assert_eq!(manager.active_lease_count(), 1);
            }
            _ => panic!("Expected Granted, got {:?}", result),
        }
    }

    #[test]
    fn test_conflict_detection() {
        let mut manager = LeaseManager::new(60);
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        let node = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![node].into_iter().collect();

        // Session A acquires lease
        match manager.acquire(session_a, region.clone(), true, "A's work".to_string()) {
            LeaseResult::Granted(_) => {}
            _ => panic!("First acquire should succeed"),
        }

        // Session B tries overlapping region
        match manager.acquire(session_b, region.clone(), true, "B's work".to_string()) {
            LeaseResult::Conflict {
                overlapping_nodes, ..
            } => {
                assert!(overlapping_nodes.contains(&node));
            }
            _ => panic!("Expected Conflict"),
        }
    }

    #[test]
    fn test_empty_region_rejected() {
        let mut manager = LeaseManager::new(60);
        let result = manager.acquire(Uuid::new_v4(), HashSet::new(), true, "Empty".to_string());
        match result {
            LeaseResult::InvalidRegion(_) => {}
            _ => panic!("Expected InvalidRegion"),
        }
    }

    #[test]
    fn test_release_frees_region() {
        let mut manager = LeaseManager::new(60);
        let session_id = Uuid::new_v4();
        let node = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![node].into_iter().collect();

        let lease_id = match manager.acquire(session_id, region, true, "Test".to_string()) {
            LeaseResult::Granted(lease) => lease.lease_id,
            _ => panic!("Expected Granted"),
        };

        assert!(manager.is_node_leased(node));
        assert!(manager.release(lease_id));
        assert!(!manager.is_node_leased(node));
        assert_eq!(manager.active_lease_count(), 0);
    }

    #[test]
    fn test_renew_extends_expiry() {
        let mut manager = LeaseManager::new(1); // 1 second TTL
        let session_id = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![Uuid::new_v4()].into_iter().collect();

        let lease_id = match manager.acquire(session_id, region, true, "Test".to_string()) {
            LeaseResult::Granted(lease) => lease.lease_id,
            _ => panic!("Expected Granted"),
        };

        // Renew with 3600 second extension
        let new_expiry = manager.renew(lease_id, Some(Duration::from_secs(3600)));
        assert!(new_expiry.is_some());
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = LeaseManager::new(0); // Immediate expiry
        let session_id = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![Uuid::new_v4()].into_iter().collect();

        // Acquire — will expire immediately
        let _ = manager.acquire(session_id, region, true, "Test".to_string());
        assert_eq!(manager.active_lease_count(), 0); // Already expired
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 1); // Should clean up the expired entry
    }

    #[test]
    fn test_session_lease_limit() {
        let mut manager = LeaseManager::with_config(60, 2); // Max 2 leases per session
        let session_id = Uuid::new_v4();

        // Acquire 2 leases (should succeed)
        for _ in 0..2 {
            let region: HashSet<Uuid> = vec![Uuid::new_v4()].into_iter().collect();
            match manager.acquire(session_id, region, true, "Test".to_string()) {
                LeaseResult::Granted(_) => {}
                _ => panic!("First 2 leases should succeed"),
            }
        }

        // Third lease should fail
        let region: HashSet<Uuid> = vec![Uuid::new_v4()].into_iter().collect();
        match manager.acquire(session_id, region, true, "Test".to_string()) {
            LeaseResult::InvalidRegion(msg) => {
                assert!(msg.contains("max"));
            }
            _ => panic!("Third lease should hit limit"),
        }
    }

    #[test]
    fn test_session_has_lease() {
        let mut manager = LeaseManager::new(60);
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        let node = Uuid::new_v4();
        let region: HashSet<Uuid> = vec![node].into_iter().collect();

        let _ = manager.acquire(session_a, region, true, "Test".to_string());

        assert!(manager.session_has_lease(session_a, node));
        assert!(!manager.session_has_lease(session_b, node));
    }
}
