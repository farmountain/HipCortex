# Intelligence Layer Integration Guide

How to add intelligence hooks to a new or existing HipCortex module.

## Overview

The intelligence layer provides three capabilities that any module can opt into:

1. **Self-Model**: Report health, check resource availability, get execution decisions
2. **World-Model**: Observe transitions, register/update entities, predict outcomes
3. **Coherence**: Validate operations, detect inconsistencies, gate writes

## Step 1: Add Optional Intelligence References

In your module struct, add optional `Arc<>` fields:

```rust
use std::sync::Arc;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::coherence::CoherenceChecker;

pub struct MyModule {
    // ... existing fields ...
    self_model: Option<Arc<SelfModel>>,
    world_model: Option<Arc<WorldModelEnhanced>>,
    coherence: Option<Arc<CoherenceChecker>>,
}
```

## Step 2: Implement Builder Methods

```rust
impl MyModule {
    pub fn with_self_model(mut self, sm: Arc<SelfModel>) -> Self {
        self.self_model = Some(sm);
        self
    }
    pub fn with_world_model(mut self, wm: Arc<WorldModelEnhanced>) -> Self {
        self.world_model = Some(wm);
        self
    }
    pub fn with_coherence(mut self, cc: Arc<CoherenceChecker>) -> Self {
        self.coherence = Some(cc);
        self
    }
}
```

## Step 3: Implement HealthReporter

```rust
use hipcortex::self_model::HealthReporter;

impl HealthReporter for MyModule {
    fn report_health(&self) -> ModuleHealth {
        ModuleHealth {
            latency_ms: /* your avg latency */,
            error_rate: /* your error rate */,
            resource_usage: /* your resource usage fraction */,
        }
    }
}
```

## Step 4: Add Intelligence Hooks to Operations

### Self-Model Gate (pre-operation)

```rust
pub fn do_operation(&mut self, ...) -> Result<(), String> {
    // 1. Self-model: can we execute?
    if let Some(ref sm) = self.self_model {
        if !sm.can_execute("my_operation", priority) {
            return Err("Operation rejected by Self-Model".into());
        }
    }
    // 2. Coherence: is this operation valid?
    if let Some(ref cc) = self.coherence {
        cc.gate_write("my_operation").map_err(|r| r.reason)?;
    }
    // 3. Execute the operation
    // ...
    Ok(())
}
```

### World-Model Observation (post-operation)

```rust
pub fn do_operation(&mut self, ...) -> Result<(), String> {
    // ... execute operation ...

    // Feed observation to world-model
    if let Some(ref wm) = self.world_model {
        wm.observe_transition(from_state, action, to_state);
    }
    Ok(())
}
```

### Resource Reporting (post-operation)

```rust
// After operation, report resource usage
if let Some(ref sm) = self.self_model {
    sm.report_resource_usage("my_operation", ResourceUsage {
        cpu_percent: measured_cpu,
        memory_mb: measured_memory,
        disk_io_mbps: measured_disk,
        network_io_mbps: measured_net,
        timestamp: Instant::now(),
    });
}
```

## Step 5: Write Integration Tests

```rust
#[test]
fn my_module_with_intelligence() {
    let sm = Arc::new(SelfModel::new());
    let cc = Arc::new(CoherenceChecker::new());

    let mut module = MyModule::new()
        .with_self_model(sm.clone())
        .with_coherence(cc.clone());

    // Operation should succeed when system is healthy
    assert!(module.do_operation().is_ok());

    // Verify health reporting works
    let health = module.report_health();
    assert!(health.compute_score() >= 0.0);
}
```

## Testing Without Intelligence

Modules must function without intelligence hooks — all intelligence fields are `Option<>`:

```rust
#[test]
fn my_module_works_without_intelligence() {
    let mut module = MyModule::new(); // No intelligence hooks
    assert!(module.do_operation().is_ok());
}
```

This ensures backward compatibility and allows incremental adoption.

## API Endpoint Pattern

When exposing intelligence via REST, follow this pattern:

```rust
// Route registration
.route("/my-module/operation", {
    let sm = self_model_arc.clone();
    let cc = coherence_arc.clone();
    post(move |Json(req): Json<serde_json::Value>| async move {
        // 1. Self-model gate
        // 2. Coherence gate
        // 3. Execute
        // 4. Return result with health/coherence context
    })
})
```

## Conventions

1. **Always optional**: Intelligence hooks are always `Option<Arc<...>>` — never required
2. **Non-blocking on None**: If intelligence is `None`, operations proceed without gating
3. **Lock order**: Coherence → WorldModel → SelfModel → Storage (outermost to innermost)
4. **Graceful degradation**: If intelligence check fails with error, log and proceed (don't crash)
5. **Health reporting**: Every module with intelligence hooks must implement `HealthReporter`
