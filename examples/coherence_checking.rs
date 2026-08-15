//! Coherence Checking Example — Cross-module consistency validation.
//!
//! Run: cargo run --example coherence_checking --no-default-features --features petgraph_backend

use hipcortex::coherence::*;

fn main() {
    println!("═══ HipCortex Coherence Checker Example ═══\n");

    let checker = CoherenceChecker::new();

    // ── 1. Check entities for inconsistencies ──────────────────────────
    println!("1. Checking entities...");
    let entities = ["customer_42", "order_17", "payment_89", "shipment_3"];
    for id in &entities {
        let reports = checker.check_entity(id).unwrap();
        if reports.is_empty() {
            println!("   {}: ✓ coherent", id);
        } else {
            for r in &reports {
                println!(
                    "   {}: ✗ {:?} — {}",
                    id, r.inconsistency_type, r.description
                );
            }
        }
    }

    // ── 2. Full consistency scan ───────────────────────────────────────
    println!("\n2. Full consistency check...");
    match checker.check_consistency() {
        Ok(reports) => {
            if reports.is_empty() {
                println!("   All modules consistent ✓");
            } else {
                println!("   Found {} inconsistencies:", reports.len());
                for r in &reports {
                    println!(
                        "     - [{:?}] {:?}: {}",
                        r.inconsistency_type, r.affected_entities, r.description
                    );
                }
            }
        }
        Err(e) => println!("   Check error: {}", e),
    }

    // ── 3. Auto-resolve conflicts ──────────────────────────────────────
    println!("\n3. Auto-resolving conflicts...");
    for strategy in &[
        ResolutionStrategy::Consensus,
        ResolutionStrategy::Recency,
        ResolutionStrategy::Confidence,
    ] {
        match checker.resolve_all(strategy.clone()) {
            Ok(results) => {
                let succeeded = results.iter().filter(|r| r.success).count();
                println!(
                    "   {:?}: {}/{} resolved",
                    strategy,
                    succeeded,
                    results.len()
                );
            }
            Err(e) => println!("   {:?}: error — {}", strategy, e),
        }
    }

    // ── 4. Enforce system invariants ───────────────────────────────────
    println!("\n4. Enforcing invariants...");
    match checker.enforce_invariants() {
        Ok(violations) => {
            if violations.is_empty() {
                println!("   All invariants hold ✓");
            } else {
                let critical = violations.iter().filter(|v| v.critical).count();
                println!(
                    "   {} violations ({} critical):",
                    violations.len(),
                    critical
                );
                for v in &violations {
                    let severity = if v.critical { "CRITICAL" } else { "WARNING" };
                    println!(
                        "     [{}] {:?}: {}",
                        severity, v.invariant_type, v.description
                    );
                }
            }
        }
        Err(e) => println!("   Invariant enforcement error: {}", e),
    }

    // ── 5. Write gating ────────────────────────────────────────────────
    println!("\n5. Testing write gate...");
    match checker.gate_write("insert_hypothetical_record") {
        Ok(()) => println!("   Write allowed ✓"),
        Err(rejection) => {
            println!("   Write BLOCKED: {}", rejection.reason);
            println!("   Inconsistencies: {}", rejection.inconsistencies.len());
            println!("   Violations: {}", rejection.violations.len());
        }
    }

    // ── 6. Coherence metrics ───────────────────────────────────────────
    println!("\n6. Coherence metrics...");
    match checker.get_metrics() {
        Ok(metrics) => {
            println!("   Total checks:      {}", metrics.total_checks);
            println!("   Inconsistencies:   {}", metrics.inconsistencies_found);
            println!(
                "   Auto-resolved:     {}",
                metrics.auto_resolutions_succeeded
            );
            println!("   Resolution fails:  {}", metrics.auto_resolutions_failed);
            println!("   Invariants checked:{}", metrics.invariants_validated);
            println!("   Invariants violated:{}", metrics.invariants_violated);
            println!("   Writes gated:      {}", metrics.writes_gated);
            println!("   Writes blocked:    {}", metrics.writes_blocked);
            println!("   Coherence score:   {:.2}/1.00", metrics.coherence_score);
        }
        Err(e) => println!("   Metrics error: {}", e),
    }

    println!("\n═══ Coherence check complete ═══");
}
