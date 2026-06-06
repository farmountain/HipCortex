// Coherence Module - Cross-module consistency checking and resolution
//
// This module ensures consistency across memory subsystems:
// - Inconsistency Detection: Identify conflicts between temporal, symbolic, procedural, and world-model
// - Automatic Resolution: Resolve conflicts using consensus, recency, or confidence strategies
// - Invariant Enforcement: Validate mathematical invariants (acyclicity, monotonicity, conservation)
// - Real-time Monitoring: Continuous coherence scoring and metrics

mod checker;
mod resolver;
mod invariants;

pub use checker::{ConsistencyChecker, InconsistencyType, InconsistencyReport};
pub use resolver::{ConflictResolver, ResolutionStrategy, ResolutionResult, ResolutionHistory};
pub use invariants::{SystemInvariants, InvariantType, InvariantViolation};

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Central Coherence Checker coordinator integrating consistency validation
pub struct CoherenceChecker {
    /// Inconsistency detector
    checker: Arc<RwLock<ConsistencyChecker>>,
    
    /// Conflict resolver with pluggable strategies
    resolver: Arc<RwLock<ConflictResolver>>,
    
    /// System invariant validator
    invariants: Arc<RwLock<SystemInvariants>>,
    
    /// Active inconsistencies (inconsistency_id -> report)
    active_inconsistencies: Arc<RwLock<HashMap<String, InconsistencyReport>>>,
    
    /// Coherence metrics
    metrics: Arc<RwLock<CoherenceMetrics>>,
    
    /// Last check timestamp
    last_check: Arc<RwLock<Instant>>,
}

/// Coherence monitoring metrics
#[derive(Debug, Clone)]
pub struct CoherenceMetrics {
    /// Total consistency checks performed
    pub total_checks: u64,

    /// Total inconsistencies found
    pub inconsistencies_found: u64,

    /// Automatic resolutions succeeded
    pub auto_resolutions_succeeded: u64,

    /// Automatic resolutions failed
    pub auto_resolutions_failed: u64,

    /// Invariants validated
    pub invariants_validated: u64,

    /// Invariants violated
    pub invariants_violated: u64,

    /// Current coherence score [0.0, 1.0] where 1.0 is perfect coherence
    pub coherence_score: f64,

    /// Synchronous write-gating checks performed
    pub writes_gated: u64,

    /// Writes blocked by synchronous gating
    pub writes_blocked: u64,
}

impl CoherenceMetrics {
    pub fn new() -> Self {
        Self {
            total_checks: 0,
            inconsistencies_found: 0,
            auto_resolutions_succeeded: 0,
            auto_resolutions_failed: 0,
            invariants_validated: 0,
            invariants_violated: 0,
            coherence_score: 1.0,
            writes_gated: 0,
            writes_blocked: 0,
        }
    }
}

/// Result of a synchronous write-gate check.
#[derive(Debug, Clone)]
pub struct WriteRejection {
    /// Inconsistencies that would result from the write
    pub inconsistencies: Vec<InconsistencyReport>,
    /// Invariant violations detected
    pub violations: Vec<InvariantViolation>,
    /// Reason summary for logging
    pub reason: String,
}

impl CoherenceChecker {
    /// Create a new Coherence Checker with default configuration
    pub fn new() -> Self {
        Self {
            checker: Arc::new(RwLock::new(ConsistencyChecker::new())),
            resolver: Arc::new(RwLock::new(ConflictResolver::new())),
            invariants: Arc::new(RwLock::new(SystemInvariants::new())),
            active_inconsistencies: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CoherenceMetrics::new())),
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    // ========================================================================
    // Inconsistency Detection
    // ========================================================================

    /// Run full consistency check across all modules
    ///
    /// Returns list of detected inconsistencies
    pub fn check_consistency(&self) -> Result<Vec<InconsistencyReport>, String> {
        let mut checker = self.checker.write()
            .map_err(|e| format!("Failed to acquire checker lock: {}", e))?;
        
        let mut metrics = self.metrics.write()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        
        // Update last check time
        let mut last_check = self.last_check.write()
            .map_err(|e| format!("Failed to acquire last_check lock: {}", e))?;
        *last_check = Instant::now();
        
        // Increment check counter
        metrics.total_checks += 1;
        
        // Run consistency checks
        let inconsistencies = checker.check_all()?;
        
        // Update metrics
        metrics.inconsistencies_found += inconsistencies.len() as u64;
        
        // Store active inconsistencies
        let mut active = self.active_inconsistencies.write()
            .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
        
        for report in &inconsistencies {
            active.insert(report.id.clone(), report.clone());
        }
        
        Ok(inconsistencies)
    }

    /// Check consistency for specific entity (targeted check)
    pub fn check_entity(&self, entity_id: &str) -> Result<Vec<InconsistencyReport>, String> {
        let mut checker = self.checker.write()
            .map_err(|e| format!("Failed to acquire checker lock: {}", e))?;
        
        checker.check_entity(entity_id)
    }

    /// Check if scheduled check is needed (60 second interval)
    pub fn should_run_scheduled_check(&self) -> Result<bool, String> {
        let last_check = self.last_check.read()
            .map_err(|e| format!("Failed to acquire last_check lock: {}", e))?;
        
        Ok(last_check.elapsed() >= Duration::from_secs(60))
    }

    // ========================================================================
    // Conflict Resolution
    // ========================================================================

    /// Automatically resolve an inconsistency using configured strategy
    ///
    /// Returns resolution result with chosen value and rationale
    pub fn resolve_conflict(
        &self,
        inconsistency: &InconsistencyReport,
        strategy: ResolutionStrategy,
    ) -> Result<ResolutionResult, String> {
        let mut resolver = self.resolver.write()
            .map_err(|e| format!("Failed to acquire resolver lock: {}", e))?;
        
        let mut metrics = self.metrics.write()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        
        // Attempt resolution
        let result = resolver.resolve(inconsistency, strategy)?;
        
        // Update metrics
        if result.success {
            metrics.auto_resolutions_succeeded += 1;
            
            // Remove from active inconsistencies if resolved
            let mut active = self.active_inconsistencies.write()
                .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
            active.remove(&inconsistency.id);
        } else {
            metrics.auto_resolutions_failed += 1;
        }
        
        Ok(result)
    }

    /// Attempt automatic resolution for all active inconsistencies
    pub fn resolve_all(&self, strategy: ResolutionStrategy) -> Result<Vec<ResolutionResult>, String> {
        let active = self.active_inconsistencies.read()
            .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
        
        let inconsistencies: Vec<_> = active.values().cloned().collect();
        drop(active);
        
        let mut results = Vec::new();
        for inconsistency in &inconsistencies {
            match self.resolve_conflict(inconsistency, strategy.clone()) {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Log error but continue resolving others
                    eprintln!("Failed to resolve inconsistency {}: {}", inconsistency.id, e);
                }
            }
        }
        
        Ok(results)
    }

    /// Apply manual override resolution for an inconsistency
    pub fn apply_manual_override(
        &self,
        inconsistency_id: &str,
        chosen_value: serde_json::Value,
        operator: &str,
    ) -> Result<(), String> {
        let mut resolver = self.resolver.write()
            .map_err(|e| format!("Failed to acquire resolver lock: {}", e))?;
        
        resolver.apply_manual_override(inconsistency_id, chosen_value, operator)?;
        
        // Remove from active inconsistencies
        let mut active = self.active_inconsistencies.write()
            .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
        active.remove(inconsistency_id);
        
        Ok(())
    }

    // ========================================================================
    // Invariant Enforcement
    // ========================================================================

    /// Validate all system invariants
    ///
    /// Returns list of violated invariants (empty if all pass)
    pub fn enforce_invariants(&self) -> Result<Vec<InvariantViolation>, String> {
        let mut invariants = self.invariants.write()
            .map_err(|e| format!("Failed to acquire invariants lock: {}", e))?;
        
        let mut metrics = self.metrics.write()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        
        let violations = invariants.validate_all()?;
        
        // Update metrics
        metrics.invariants_validated += 1;
        metrics.invariants_violated += violations.len() as u64;
        
        // If critical invariant violated, halt further operations
        for violation in &violations {
            if violation.critical {
                eprintln!("CRITICAL INVARIANT VIOLATION: {:?}", violation);
                // In production, this would trigger system health degradation
            }
        }
        
        Ok(violations)
    }

    // ========================================================================
    // Synchronous Write-Gating
    // ========================================================================

    /// Synchronously validate a write operation before it executes.
    ///
    /// Runs invariants check (decay monotonicity, conservation, acyclicity) and
    /// returns Ok(()) if the write is safe, or Err(WriteRejection) if it would
    /// violate system invariants.
    ///
    /// This is the synchronous safety gate — unlike the async 60s background
    /// checker, this blocks the write BEFORE it happens.
    pub fn gate_write(&self, context: &str) -> Result<(), WriteRejection> {
        let mut metrics = self.metrics.write()
            .map_err(|_| WriteRejection {
                inconsistencies: vec![],
                violations: vec![],
                reason: "failed to acquire metrics lock".into(),
            })?;

        metrics.writes_gated += 1;
        drop(metrics);

        // Run invariant validation synchronously
        match self.enforce_invariants() {
            Ok(violations) => {
                let critical: Vec<_> = violations.into_iter()
                    .filter(|v| v.critical)
                    .collect();

                if !critical.is_empty() {
                    if let Ok(mut metrics) = self.metrics.write() {
                        metrics.writes_blocked += 1;
                    }

                    return Err(WriteRejection {
                        reason: format!(
                            "{} critical invariant(s) violated: {}",
                            critical.len(),
                            critical.iter()
                                .map(|v| format!("{:?}", v.invariant_type))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        violations: critical,
                        inconsistencies: vec![],
                    });
                }
                Ok(())
            }
            Err(e) => Err(WriteRejection {
                inconsistencies: vec![],
                violations: vec![],
                reason: format!("invariant check failed: {}", e),
            }),
        }
    }

    /// Check consistency and auto-resolve if possible. If resolution fails,
    /// return the write rejection. Used for writes that need coherence gating
    /// but allow auto-resolution before blocking.
    pub fn check_and_resolve(&self, strategy: ResolutionStrategy) -> Result<(), WriteRejection> {
        // Run consistency check
        let inconsistencies = self.check_consistency().map_err(|e| WriteRejection {
            inconsistencies: vec![],
            violations: vec![],
            reason: format!("consistency check failed: {}", e),
        })?;

        if inconsistencies.is_empty() {
            return Ok(());
        }

        // Try auto-resolution
        let results = self.resolve_all(strategy).map_err(|e| WriteRejection {
            inconsistencies: vec![],
            violations: vec![],
            reason: format!("auto-resolution failed: {}", e),
        })?;

        let unresolved: Vec<_> = results.into_iter()
            .filter(|r| !r.success)
            .collect();

        if !unresolved.is_empty() {
            // Also run invariants for a complete picture
            let violations = self.enforce_invariants().unwrap_or_default();

            if let Ok(mut metrics) = self.metrics.write() {
                metrics.writes_blocked += 1;
            }

            return Err(WriteRejection {
                reason: format!(
                    "{} inconsistencies could not be auto-resolved",
                    unresolved.len()
                ),
                inconsistencies: vec![], // already stored in active_inconsistencies
                violations,
            });
        }

        Ok(())
    }

    /// Get write-gating statistics.
    pub fn get_gate_stats(&self) -> Result<(u64, u64), String> {
        let metrics = self.metrics.read()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        Ok((metrics.writes_gated, metrics.writes_blocked))
    }

    /// Validate specific invariant type
    pub fn check_invariant(&self, invariant: InvariantType) -> Result<Option<InvariantViolation>, String> {
        let mut invariants = self.invariants.write()
            .map_err(|e| format!("Failed to acquire invariants lock: {}", e))?;
        
        invariants.validate_specific(invariant)
    }

    // ========================================================================
    // Coherence Metrics
    // ========================================================================

    /// Compute current coherence score
    ///
    /// coherence_score = 1.0 - (active_inconsistencies / total_entities)
    /// where 1.0 is perfect coherence
    pub fn compute_coherence_score(&self, total_entities: usize) -> Result<f64, String> {
        let active = self.active_inconsistencies.read()
            .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
        
        let active_count = active.len();
        
        let score = if total_entities == 0 {
            1.0 // Perfect coherence if no entities
        } else {
            1.0 - (active_count as f64 / total_entities as f64)
        };
        
        // Update cached score
        let mut metrics = self.metrics.write()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        metrics.coherence_score = score;
        
        Ok(score)
    }

    /// Get current coherence metrics
    pub fn get_metrics(&self) -> Result<CoherenceMetrics, String> {
        let metrics = self.metrics.read()
            .map_err(|e| format!("Failed to acquire metrics lock: {}", e))?;
        
        Ok(metrics.clone())
    }

    /// Get all active inconsistencies
    pub fn get_active_inconsistencies(&self) -> Result<Vec<InconsistencyReport>, String> {
        let active = self.active_inconsistencies.read()
            .map_err(|e| format!("Failed to acquire active_inconsistencies lock: {}", e))?;
        
        Ok(active.values().cloned().collect())
    }

    /// Get resolution history
    pub fn get_resolution_history(&self) -> Result<Vec<ResolutionHistory>, String> {
        let resolver = self.resolver.read()
            .map_err(|e| format!("Failed to acquire resolver lock: {}", e))?;
        
        resolver.get_history()
    }
}

impl Default for CoherenceChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coherence_checker_creation() {
        let checker = CoherenceChecker::new();
        let metrics = checker.get_metrics().unwrap();
        assert_eq!(metrics.total_checks, 0);
        assert_eq!(metrics.coherence_score, 1.0);
    }

    #[test]
    fn test_scheduled_check_timing() {
        let checker = CoherenceChecker::new();
        
        // Should not need check immediately
        assert!(!checker.should_run_scheduled_check().unwrap());
    }

    #[test]
    fn test_coherence_score_computation() {
        let checker = CoherenceChecker::new();
        
        // Perfect coherence with no inconsistencies
        let score = checker.compute_coherence_score(100).unwrap();
        assert_eq!(score, 1.0);
        
        // Perfect coherence with zero entities
        let score = checker.compute_coherence_score(0).unwrap();
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_active_inconsistencies_tracking() {
        let checker = CoherenceChecker::new();
        
        // Initially no inconsistencies
        let active = checker.get_active_inconsistencies().unwrap();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_metrics_initialization() {
        let metrics = CoherenceMetrics::new();
        assert_eq!(metrics.total_checks, 0);
        assert_eq!(metrics.inconsistencies_found, 0);
        assert_eq!(metrics.auto_resolutions_succeeded, 0);
        assert_eq!(metrics.auto_resolutions_failed, 0);
        assert_eq!(metrics.invariants_validated, 0);
        assert_eq!(metrics.invariants_violated, 0);
        assert_eq!(metrics.coherence_score, 1.0);
    }
}
