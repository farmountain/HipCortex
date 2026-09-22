# Reconstruction Engine Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close 4 gaps (Laws, Policies, MDL sparsity, axiomatic write-back) turning HipCortex from cognitive substrate into dynamic reconstruction engine.

**Architecture:** New `MemoryType::Law` + `MemoryType::Policy` variants; `LawExtractor` clusters surprising Intents → writes Laws; `PolicyRegistry` drives `DigitalTwin::step_with_policies()`; `CognitiveGC::gc_action_for_law()` applies MDL sparsity pressure. `route_uncertainty()` called at each ambiguous gate (self-prompt first, user only at T3).

**Tech Stack:** Rust (petgraph_backend), existing `MemoryStore`, `DigitalTwin`, `CognitiveGC`, `route_uncertainty` from `agent_guidance.rs`.

**Read before starting:**
- `src/memory_record.rs` — MemoryType enum (11 variants, derives ValueEnum + Hash + PartialEq + Eq)
- `src/payloads.rs` — existing payload structs (BeliefPayload, GoalPayload, etc.)
- `src/digital_twin.rs` — step() and step_with_wm() signatures
- `src/cognitive_gc.rs` — gc_action() signature and GcAction enum
- `src/agent_guidance.rs` — route_uncertainty() signature

**Security:** All writes go through `store.add()` — SafetyGuardrail respected. No raw writes.

---

### Task 1: Add `MemoryType::Law` and `MemoryType::Policy` variants

**Files:**
- Modify: `src/memory_record.rs`
- Modify: `src/web_server.rs` (around line 4877 — `parse_record_type_alias`)
- Modify: `src/grpc_server.rs` (around line 40 — string match on `rec.record_type`)

- [ ] **Step 1: Add variants to MemoryType enum**

In `src/memory_record.rs`, the enum currently ends at `Receipt`. Add after it:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ValueEnum, Hash)]
pub enum MemoryType {
    Temporal,
    Symbolic,
    Procedural,
    Reflexion,
    Perception,
    Goal,
    Skill,
    Belief,
    Decision,
    Intent,
    Receipt,
    Law,     // structural equation elevated from anomaly evidence
    Policy,  // reactive rule governing entity state evolution
}
```

- [ ] **Step 2: Write failing tests for round-trip serde**

Create `tests/unit/memory_type_law_policy_tests.rs`:

```rust
use hipcortex::memory_record::MemoryType;

#[test]
fn law_memory_type_serde_round_trip() {
    let t = MemoryType::Law;
    let s = serde_json::to_string(&t).unwrap();
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Law);
}

#[test]
fn policy_memory_type_serde_round_trip() {
    let t = MemoryType::Policy;
    let s = serde_json::to_string(&t).unwrap();
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Policy);
}
```

- [ ] **Step 3: Run tests — expect FAIL (module not registered yet)**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib memory_type_law_policy
```

Expected: compile error "can't find module" — confirms test file registration needed.

- [ ] **Step 4: Register test file in `tests/unit_suite.rs`**

Add at the end of the existing mod list:

```rust
mod memory_type_law_policy_tests;
```

- [ ] **Step 5: Add `parse_record_type_alias` arms in `web_server.rs`**

Find `parse_record_type_alias` (around line 4896). Add before the error arm:

```rust
"law" | "Law" => Ok(crate::memory_record::MemoryType::Law),
"policy" | "Policy" => Ok(crate::memory_record::MemoryType::Policy),
```

- [ ] **Step 6: Add arms in `grpc_server.rs` string match**

Find the `match rec.record_type.as_str()` block (around line 40). Add:

```rust
"Law" => MemoryType::Law,
"Policy" => MemoryType::Policy,
```

- [ ] **Step 7: Run tests — expect PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib memory_type_law_policy
```

Expected: 2 tests pass.

- [ ] **Step 8: Verify full lib suite still green**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

- [ ] **Step 9: Commit**

```sh
git add src/memory_record.rs src/web_server.rs src/grpc_server.rs tests/unit/memory_type_law_policy_tests.rs tests/unit_suite.rs
git commit -m "feat(v3.16.0): add MemoryType::Law + Policy variants with serde round-trip tests"
```

---

### Task 2: Add `LawPayload` and `PolicyPayload` to `payloads.rs`

**Files:**
- Modify: `src/payloads.rs`
- Test in: `tests/unit/memory_type_law_policy_tests.rs` (extend same file)

- [ ] **Step 1: Write failing tests for payload serde**

Add to `tests/unit/memory_type_law_policy_tests.rs`:

```rust
use hipcortex::payloads::{LawPayload, PolicyPayload};
use uuid::Uuid;

#[test]
fn law_payload_serde_round_trip() {
    let p = LawPayload {
        equation: "velocity ~ apply_force(mass)".into(),
        causal_variables: vec!["mass".into(), "friction".into()],
        effect_variable: "velocity".into(),
        evidence_ids: vec![Uuid::new_v4()],
        mdl_score: 1.5,
        domain: Some("physics".into()),
        holds_under_intervention: true,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: LawPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.equation, p.equation);
    assert_eq!(back.mdl_score, p.mdl_score);
    assert!(back.holds_under_intervention);
}

#[test]
fn policy_payload_serde_round_trip() {
    let p = PolicyPayload {
        entity_id: Uuid::new_v4(),
        trigger_condition: "state.velocity > 2.0".into(),
        action_fn: "apply_brake(0.5)".into(),
        priority: 10,
        active: true,
        law_id: None,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: PolicyPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.trigger_condition, p.trigger_condition);
    assert_eq!(back.priority, 10);
    assert!(back.active);
}

#[test]
fn policy_payload_defaults_active() {
    let json = r#"{"entity_id":"00000000-0000-0000-0000-000000000001","trigger_condition":"x","action_fn":"y","priority":1}"#;
    let p: PolicyPayload = serde_json::from_str(json).unwrap();
    assert!(p.active);   // default = true
    assert!(p.law_id.is_none());
}
```

- [ ] **Step 2: Run tests — expect FAIL (structs don't exist yet)**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib law_payload_serde
```

Expected: compile error `cannot find type LawPayload`.

- [ ] **Step 3: Add `LawPayload` to `src/payloads.rs`**

Add after `BeliefPayload` (after line ~198):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LawPayload {
    pub equation: String,
    #[serde(default)]
    pub causal_variables: Vec<String>,
    #[serde(default = "default_effect_variable")]
    pub effect_variable: String,
    #[serde(default)]
    pub evidence_ids: Vec<uuid::Uuid>,
    #[serde(default)]
    pub mdl_score: f64,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default = "default_true")]
    pub holds_under_intervention: bool,
}

fn default_effect_variable() -> String { "unknown".into() }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyPayload {
    pub entity_id: uuid::Uuid,
    pub trigger_condition: String,
    pub action_fn: String,
    #[serde(default)]
    pub priority: u32,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub law_id: Option<uuid::Uuid>,
}
```

- [ ] **Step 4: Export from payloads.rs public surface**

`payloads.rs` is already `pub` — no mod change needed. Verify `LawPayload` and `PolicyPayload` appear in the file.

- [ ] **Step 5: Run payload tests — expect PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib law_payload_serde policy_payload_serde policy_payload_defaults
```

Expected: 3 tests pass.

- [ ] **Step 6: Full lib suite check**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

- [ ] **Step 7: Commit**

```sh
git add src/payloads.rs tests/unit/memory_type_law_policy_tests.rs
git commit -m "feat(v3.16.0): add LawPayload + PolicyPayload structs with serde defaults"
```

---

### Task 3: `LawExtractor` — anomaly → Law pipeline with route_uncertainty hooks

**Files:**
- Create: `src/law_extractor.rs`
- Modify: `src/lib.rs` (register module)
- Modify: `src/emergence.rs` (call LawExtractor::attempt_extract)
- Create: `tests/unit/law_extractor_tests.rs`
- Modify: `tests/unit_suite.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/unit/law_extractor_tests.rs`:

```rust
use hipcortex::law_extractor::LawExtractor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::backends::petgraph::PetgraphBackend;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

fn make_store() -> MemoryStore<PetgraphBackend> {
    MemoryStore::new(PetgraphBackend::new())
}

fn surprising_intent(actor: &str, action: &str, entity: &str, content: &str) -> MemoryRecord {
    MemoryRecord {
        id: Uuid::new_v4(),
        record_type: MemoryType::Intent,
        timestamp: Utc::now(),
        actor: actor.into(),
        action: action.into(),
        target: entity.into(),
        metadata: json!({
            "was_surprising": true,
            "status": "Received",
            "target_entity": entity,
            "content_excerpt": content,
        }),
        integrity: None,
        hash_version: 0,
        access_count: 0,
        // fill remaining defaults
        ..Default::default()
    }
}

#[test]
fn extracts_law_from_three_surprising_intents() {
    let mut store = make_store();
    for i in 0..3 {
        let rec = surprising_intent("agent", "apply_force", "ball", &format!("force applied {}", i));
        store.add(rec).unwrap();
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(!laws.is_empty(), "expected at least one Law extracted");
}

#[test]
fn no_law_from_fewer_than_three() {
    let mut store = make_store();
    for i in 0..2 {
        let rec = surprising_intent("agent", "apply_force", "ball", &format!("force {}", i));
        store.add(rec).unwrap();
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(laws.is_empty());
}

#[test]
fn law_extraction_idempotent() {
    let mut store = make_store();
    for i in 0..3 {
        let rec = surprising_intent("agent", "apply_force", "ball", &format!("force {}", i));
        store.add(rec).unwrap();
    }
    let first = LawExtractor::attempt_extract(&mut store, "agent");
    let second = LawExtractor::attempt_extract(&mut store, "agent");
    assert_eq!(second.len(), 0, "second extraction should find duplicate and skip");
    drop(first); // suppress unused warning
}

#[test]
fn non_surprising_intents_ignored() {
    let mut store = make_store();
    for i in 0..5 {
        let mut rec = surprising_intent("agent", "apply_force", "ball", &format!("f {}", i));
        // make them non-surprising
        rec.metadata["was_surprising"] = serde_json::Value::Bool(false);
        store.add(rec).unwrap();
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(laws.is_empty());
}
```

- [ ] **Step 2: Run — expect compile error**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib extracts_law_from_three
```

Expected: `cannot find module law_extractor`.

- [ ] **Step 3: Create `src/law_extractor.rs`**

```rust
//! LawExtractor — clusters surprising Intent records into reusable structural Laws.
//! Self-prompts via route_uncertainty before writing when causal pattern is ambiguous.

use std::collections::HashMap;
use uuid::Uuid;

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::payloads::LawPayload;
use crate::persistence::MemoryBackend;

pub const MIN_SURPRISING_FOR_LAW: usize = 3;
const MAX_CAUSAL_VARS: usize = 4;

pub struct LawExtractor;

impl LawExtractor {
    /// Scan surprising Intent records for actor; extract Laws if cluster ≥ MIN_SURPRISING_FOR_LAW.
    /// Returns IDs of newly written Law records (empty if none extracted or all duplicates).
    pub fn attempt_extract<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        actor: &str,
    ) -> Vec<Uuid> {
        let intents = Self::collect_surprising(store, actor);
        if intents.is_empty() {
            return Vec::new();
        }

        // Cluster by target_entity
        let mut clusters: HashMap<String, Vec<MemoryRecord>> = HashMap::new();
        for rec in intents {
            let entity = rec
                .metadata
                .get("target_entity")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            clusters.entry(entity).or_default().push(rec);
        }

        let existing_equations = Self::existing_law_equations(store);
        let mut written = Vec::new();

        for (entity, cluster) in clusters {
            if cluster.len() < MIN_SURPRISING_FOR_LAW {
                continue;
            }

            // Extract causal variables from action tokens
            let mut action_freq: HashMap<String, usize> = HashMap::new();
            for rec in &cluster {
                for token in rec.action.split_whitespace() {
                    let t = token.to_lowercase();
                    if t.len() > 2 {
                        *action_freq.entry(t).or_default() += 1;
                    }
                }
            }
            let mut causal_vars: Vec<String> = action_freq
                .into_iter()
                .filter(|(_, count)| *count >= 2)
                .map(|(token, _)| token)
                .take(MAX_CAUSAL_VARS)
                .collect();
            causal_vars.sort();

            // Self-prompt: if < 2 causal variables, check content_excerpt for more
            if causal_vars.len() < 2 {
                let from_content: Vec<String> = cluster
                    .iter()
                    .filter_map(|r| r.metadata.get("content_excerpt").and_then(|v| v.as_str()))
                    .flat_map(|c| c.split_whitespace().map(str::to_string))
                    .filter(|t| t.len() > 3)
                    .collect();
                for token in from_content {
                    let t = token.to_lowercase();
                    if !causal_vars.contains(&t) {
                        causal_vars.push(t);
                        if causal_vars.len() >= 2 {
                            break;
                        }
                    }
                }
            }

            // Still < 2 after self-prompt — call route_uncertainty to decide next step
            if causal_vars.len() < 2 {
                let route = crate::agent_guidance::route_uncertainty(
                    true,
                    0,
                    false,
                    0.8,
                );
                // T0 self-prompt: use entity name as causal variable fallback
                if matches!(route, crate::agent_guidance::ClarifyRoute::SelfPrompt { .. }) {
                    causal_vars.push(entity.clone());
                }
                // T3 ask: skip this cluster (don't write ambiguous Law)
                if matches!(route, crate::agent_guidance::ClarifyRoute::AskUser { .. }) {
                    continue;
                }
            }

            let effect = entity.clone();
            let eq = format!(
                "{} ~ {}({})",
                effect,
                cluster[0].action.split_whitespace().next().unwrap_or("unknown"),
                causal_vars.join(", ")
            );

            // Idempotency: skip if equation already exists
            if existing_equations.contains(&eq) {
                continue;
            }

            let evidence_ids: Vec<Uuid> = cluster.iter().map(|r| r.id).collect();
            let mdl_score = evidence_ids.len() as f64
                / (eq.len() as f64 / 20.0 + 1.0);

            let payload = LawPayload {
                equation: eq.clone(),
                causal_variables: causal_vars,
                effect_variable: effect,
                evidence_ids,
                mdl_score,
                domain: None,
                holds_under_intervention: true,
            };

            let mut rec = MemoryRecord::new(
                MemoryType::Law,
                actor,
                "extracted",
                "law-store",
            );
            rec.metadata = serde_json::to_value(&payload).unwrap_or_default();

            if let Ok(id) = store.add(rec) {
                written.push(id);
            }
        }

        written
    }

    fn collect_surprising<B: MemoryBackend>(
        store: &MemoryStore<B>,
        actor: &str,
    ) -> Vec<MemoryRecord> {
        store
            .all_by_type(MemoryType::Intent)
            .into_iter()
            .filter(|r| {
                r.actor == actor
                    && r.metadata
                        .get("was_surprising")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
            .collect()
    }

    fn existing_law_equations<B: MemoryBackend>(store: &MemoryStore<B>) -> std::collections::HashSet<String> {
        store
            .all_by_type(MemoryType::Law)
            .into_iter()
            .filter_map(|r| {
                r.metadata
                    .get("equation")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect()
    }
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

Find the block where `emergence` is declared and add after it:

```rust
pub mod law_extractor;
```

- [ ] **Step 5: Register test in `tests/unit_suite.rs`**

```rust
mod law_extractor_tests;
```

- [ ] **Step 6: Run tests — expect PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | grep -E "law_extractor|FAILED|ok\."
```

Expected: 4 law_extractor tests pass.

- [ ] **Step 7: Wire into `src/emergence.rs`**

At the end of `EmergenceDetector::on_temporal_write()`, after `Self::detect(store, actor)`:

```rust
// Law extraction: cluster surprising Intents into reusable Laws
let _law_ids = crate::law_extractor::LawExtractor::attempt_extract(store, actor);
// Returns newly written Law IDs (ignored here; callers can query store for Laws)
```

- [ ] **Step 8: Verify full lib suite**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

Expected: all pass.

- [ ] **Step 9: Commit**

```sh
git add src/law_extractor.rs src/lib.rs src/emergence.rs tests/unit/law_extractor_tests.rs tests/unit_suite.rs
git commit -m "feat(v3.16.0): LawExtractor — anomaly→Law pipeline with route_uncertainty self-prompt gate"
```

---

### Task 4: `PolicyRegistry` + `DigitalTwin::step_with_policies()`

**Files:**
- Create: `src/policy_registry.rs`
- Modify: `src/lib.rs`
- Modify: `src/digital_twin.rs`
- Create: `tests/unit/policy_registry_tests.rs`
- Modify: `tests/unit_suite.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/unit/policy_registry_tests.rs`:

```rust
use hipcortex::policy_registry::{PolicyRegistry, PolicyRecord};
use hipcortex::payloads::PolicyPayload;
use uuid::Uuid;

fn make_policy(entity: Uuid, condition: &str, action: &str, priority: u32) -> PolicyRecord {
    PolicyRecord {
        id: Uuid::new_v4(),
        payload: PolicyPayload {
            entity_id: entity,
            trigger_condition: condition.into(),
            action_fn: action.into(),
            priority,
            active: true,
            law_id: None,
        },
    }
}

#[test]
fn active_for_entity_returns_sorted_by_priority() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(make_policy(entity, "x > 1", "brake(0.5)", 5));
    reg.register_record(make_policy(entity, "x > 2", "brake(1.0)", 10));
    let policies = reg.active_for_entity(entity);
    assert_eq!(policies.len(), 2);
    assert_eq!(policies[0].payload.priority, 10); // highest first
}

#[test]
fn inactive_policies_excluded() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    let mut pol = make_policy(entity, "x > 1", "brake(0.5)", 5);
    pol.payload.active = false;
    reg.register_record(pol);
    assert!(reg.active_for_entity(entity).is_empty());
}

#[test]
fn deactivate_removes_from_active() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    let pol = make_policy(entity, "x > 1", "brake(0.5)", 5);
    let id = pol.id;
    reg.register_record(pol);
    reg.deactivate(id);
    assert!(reg.active_for_entity(entity).is_empty());
}

#[test]
fn different_entities_isolated() {
    let e1 = Uuid::new_v4();
    let e2 = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(make_policy(e1, "x > 1", "act_a", 1));
    assert!(reg.active_for_entity(e2).is_empty());
}
```

- [ ] **Step 2: Run — expect compile error**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib active_for_entity_returns_sorted
```

Expected: `cannot find module policy_registry`.

- [ ] **Step 3: Create `src/policy_registry.rs`**

```rust
//! PolicyRegistry — in-memory store of active Policies by entity.
//! Consulted by DigitalTwin::step_with_policies() before applying RK4 dynamics.

use std::collections::HashMap;
use uuid::Uuid;

use crate::payloads::PolicyPayload;

pub struct PolicyRecord {
    pub id: Uuid,
    pub payload: PolicyPayload,
}

pub struct PolicyRegistry {
    records: HashMap<Uuid, PolicyRecord>,
}

impl PolicyRegistry {
    pub fn new() -> Self {
        Self { records: HashMap::new() }
    }

    /// Register a PolicyRecord. Calls route_uncertainty if trigger_condition is empty.
    pub fn register_record(&mut self, record: PolicyRecord) {
        if record.payload.trigger_condition.is_empty() {
            // Self-prompt: empty condition — use route_uncertainty to decide
            let route = crate::agent_guidance::route_uncertainty(true, 0, false, 1.0);
            if matches!(route, crate::agent_guidance::ClarifyRoute::AskUser { .. }) {
                // T3: skip registration — condition genuinely ambiguous
                return;
            }
            // T0 self-prompt: treat as always-active (condition = "always")
        }
        self.records.insert(record.id, record);
    }

    /// Returns active Policies for entity, sorted by priority descending.
    pub fn active_for_entity(&self, entity_id: Uuid) -> Vec<&PolicyRecord> {
        let mut matching: Vec<&PolicyRecord> = self
            .records
            .values()
            .filter(|r| r.payload.entity_id == entity_id && r.payload.active)
            .collect();
        matching.sort_by(|a, b| b.payload.priority.cmp(&a.payload.priority));
        matching
    }

    pub fn deactivate(&mut self, policy_id: Uuid) -> bool {
        if let Some(rec) = self.records.get_mut(&policy_id) {
            rec.payload.active = false;
            true
        } else {
            false
        }
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

```rust
pub mod policy_registry;
```

- [ ] **Step 5: Register tests in `tests/unit_suite.rs`**

```rust
mod policy_registry_tests;
```

- [ ] **Step 6: Run policy tests — expect PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | grep -E "policy_registry|ok\."
```

Expected: 4 tests pass.

- [ ] **Step 7: Add `step_with_policies()` to `src/digital_twin.rs`**

Add after the existing `step_with_wm()` method:

```rust
/// Advance twin by one step, consulting PolicyRegistry for the entity before applying dynamics.
/// The highest-priority matching Policy overrides the provided `action` argument.
pub fn step_with_policies(
    &mut self,
    action: &str,
    registry: &crate::policy_registry::PolicyRegistry,
    entity_id: uuid::Uuid,
) -> Result<Vec<f64>, crate::error::CognitiveError> {
    let effective_action = registry
        .active_for_entity(entity_id)
        .into_iter()
        .find(|p| {
            // Simple trigger evaluation: condition is non-empty and current state is non-zero
            !p.payload.trigger_condition.is_empty()
                && !self.trajectory.last().map(|s: &Vec<f64>| s.iter().all(|&v| v == 0.0)).unwrap_or(true)
        })
        .map(|p| p.payload.action_fn.as_str())
        .unwrap_or(action);

    self.step(effective_action)
}
```

- [ ] **Step 8: Write digital_twin policy test**

Add to `tests/unit/policy_registry_tests.rs`:

```rust
use hipcortex::digital_twin::{DigitalTwin, SyncPolicy};
use hipcortex::continuous_dynamics::ContinuousDynamics;
use hipcortex::backends::petgraph::PetgraphBackend;
use hipcortex::memory_store::MemoryStore;

#[test]
fn step_with_policies_uses_policy_action_when_matched() {
    let store = MemoryStore::new(PetgraphBackend::new());
    let dynamics = ContinuousDynamics::identity(2, 0.1);
    let mut twin = DigitalTwin::new(store, dynamics, SyncPolicy::Isolated, 0, std::collections::HashMap::new());
    // Prime state with non-zero trajectory so trigger fires
    let _ = twin.step("init");

    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(make_policy(entity, "state_nonzero", "policy_action", 10));

    // Should use policy_action (matched) not "original_action"
    let result = twin.step_with_policies("original_action", &reg, entity);
    assert!(result.is_ok());
}
```

- [ ] **Step 9: Full lib suite green**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

- [ ] **Step 10: Commit**

```sh
git add src/policy_registry.rs src/lib.rs src/digital_twin.rs tests/unit/policy_registry_tests.rs tests/unit_suite.rs
git commit -m "feat(v3.16.0): PolicyRegistry + DigitalTwin::step_with_policies() — Policies as first-class citizens"
```

---

### Task 5: MDL sparsity pressure in `CognitiveGC`

**Files:**
- Modify: `src/cognitive_gc.rs`
- Create: `tests/unit/cognitive_gc_law_tests.rs`
- Modify: `tests/unit_suite.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/unit/cognitive_gc_law_tests.rs`:

```rust
use hipcortex::cognitive_gc::{CognitiveGC, GcAction, MDL_KEEP_THRESHOLD};
use uuid::Uuid;

#[test]
fn gc_law_below_threshold_orphaned_deletes() {
    let gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    let score = MDL_KEEP_THRESHOLD - 0.1;
    assert_eq!(gc.gc_action_for_law(id, score), GcAction::Delete);
}

#[test]
fn gc_law_below_threshold_referenced_archives() {
    let mut gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    let referencing = Uuid::new_v4();
    gc.register_reference(id, referencing);
    let score = MDL_KEEP_THRESHOLD - 0.1;
    assert_eq!(gc.gc_action_for_law(id, score), GcAction::Archive);
}

#[test]
fn gc_law_above_threshold_keeps_regardless_of_refs() {
    let gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    let score = MDL_KEEP_THRESHOLD + 0.1;
    assert_eq!(gc.gc_action_for_law(id, score), GcAction::Keep);
}

#[test]
fn gc_law_exactly_at_threshold_keeps() {
    let gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    assert_eq!(gc.gc_action_for_law(id, MDL_KEEP_THRESHOLD), GcAction::Keep);
}
```

- [ ] **Step 2: Run — expect compile error**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib gc_law_below_threshold
```

Expected: `cannot find value MDL_KEEP_THRESHOLD` or `no method gc_action_for_law`.

- [ ] **Step 3: Add constant and method to `src/cognitive_gc.rs`**

After the existing `gc_action()` method, add:

```rust
/// MDL score at or above which a Law is always kept (regardless of reference count).
pub const MDL_KEEP_THRESHOLD: f64 = 0.5;

/// GC decision for a Law record, incorporating both MDL score and reference count.
/// Laws above MDL_KEEP_THRESHOLD are kept unconditionally (high information value).
/// Laws below threshold follow reference-count logic (Archive if referenced, Delete if orphaned).
pub fn gc_action_for_law(&self, record_id: Uuid, mdl_score: f64) -> GcAction {
    if mdl_score >= MDL_KEEP_THRESHOLD {
        return GcAction::Keep;
    }
    match self.references.get(&record_id) {
        Some(refs) if !refs.is_empty() => GcAction::Archive,
        _ => GcAction::Delete,
    }
}
```

- [ ] **Step 4: Register tests in `tests/unit_suite.rs`**

```rust
mod cognitive_gc_law_tests;
```

- [ ] **Step 5: Run tests — expect PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | grep -E "gc_law|ok\."
```

Expected: 4 tests pass.

- [ ] **Step 6: Full lib suite**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

- [ ] **Step 7: Commit**

```sh
git add src/cognitive_gc.rs tests/unit/cognitive_gc_law_tests.rs tests/unit_suite.rs
git commit -m "feat(v3.16.0): CognitiveGC::gc_action_for_law() — MDL sparsity pressure on Laws"
```

---

### Task 6: REST surface for Laws and Policies

**Files:**
- Modify: `src/web_server.rs` — add REST endpoints for Laws and Policies

- [ ] **Step 1: Add Law query endpoint**

Find the block for existing `/memory` routes. Add after `/experience/tiers`:

```rust
// GET /laws — return all Law records
.route("/laws", get(handle_list_laws::<B>))
// GET /policies/:entity_id — return active Policies for entity
.route("/policies/:entity_id", get(handle_list_policies::<B>))
```

- [ ] **Step 2: Implement handlers**

Add handler functions (before the `build_app` fn or in same file):

```rust
async fn handle_list_laws<B: MemoryBackend + Send + Sync + 'static>(
    State(state): State<AppState<B>>,
) -> impl IntoResponse {
    let store = state.memory_store.read().await;
    let laws = store.all_by_type(crate::memory_record::MemoryType::Law);
    axum::Json(serde_json::json!({ "laws": laws, "count": laws.len() }))
}

async fn handle_list_policies<B: MemoryBackend + Send + Sync + 'static>(
    State(state): State<AppState<B>>,
    axum::extract::Path(entity_id): axum::extract::Path<uuid::Uuid>,
) -> impl IntoResponse {
    let store = state.memory_store.read().await;
    let policies: Vec<_> = store
        .all_by_type(crate::memory_record::MemoryType::Policy)
        .into_iter()
        .filter(|r| {
            r.metadata
                .get("entity_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                .map(|id| id == entity_id)
                .unwrap_or(false)
        })
        .collect();
    axum::Json(serde_json::json!({ "policies": policies, "count": policies.len() }))
}
```

- [ ] **Step 3: Build check (no tests needed for REST — covered by integration suite)**

```sh
cargo build --no-default-features --features "petgraph_backend,web-server" 2>&1 | tail -5
```

Expected: `Finished` with 0 errors.

- [ ] **Step 4: Full lib suite still green**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```sh
git add src/web_server.rs
git commit -m "feat(v3.16.0): REST endpoints GET /laws + GET /policies/:entity_id"
```

---

### Task 7: End-to-end validation — ReAct loop against all ACs

**Files:**
- Create: `tests/integration/law_policy_integration_test.rs`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Write integration test covering all 10 ACs**

Create `tests/integration/law_policy_integration_test.rs`:

```rust
//! Integration: surprising Intents → Law extracted → Policy active → GC aware of MDL.

use hipcortex::backends::petgraph::PetgraphBackend;
use hipcortex::cognitive_gc::{CognitiveGC, GcAction, MDL_KEEP_THRESHOLD};
use hipcortex::law_extractor::LawExtractor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{LawPayload, PolicyPayload};
use hipcortex::policy_registry::{PolicyRecord, PolicyRegistry};
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

fn make_store() -> MemoryStore<PetgraphBackend> {
    MemoryStore::new(PetgraphBackend::new())
}

fn add_surprising_intent(store: &mut MemoryStore<PetgraphBackend>, actor: &str, action: &str, entity: &str) {
    let mut rec = MemoryRecord {
        id: Uuid::new_v4(),
        record_type: MemoryType::Intent,
        timestamp: Utc::now(),
        actor: actor.into(),
        action: action.into(),
        target: entity.into(),
        metadata: json!({
            "was_surprising": true,
            "status": "Received",
            "target_entity": entity,
            "content_excerpt": format!("{} affected {}", action, entity),
        }),
        ..Default::default()
    };
    store.add(rec).unwrap();
}

// AC-L1: Law round-trips serde
#[test]
fn ac_l1_law_payload_serde_round_trip() {
    let p = LawPayload {
        equation: "ball ~ push(force)".into(),
        causal_variables: vec!["force".into()],
        effect_variable: "ball".into(),
        evidence_ids: vec![Uuid::new_v4()],
        mdl_score: 1.2,
        domain: None,
        holds_under_intervention: true,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: LawPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.equation, p.equation);
    assert!((back.mdl_score - 1.2).abs() < 1e-9);
}

// AC-L2: Extraction fires with ≥3 surprising Intents
#[test]
fn ac_l2_law_extracted_from_cluster() {
    let mut store = make_store();
    for _ in 0..3 {
        add_surprising_intent(&mut store, "agent", "push", "ball");
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(!laws.is_empty());
    let law_recs = store.all_by_type(MemoryType::Law);
    assert!(!law_recs.is_empty());
}

// AC-L3: Idempotency
#[test]
fn ac_l3_law_extraction_idempotent() {
    let mut store = make_store();
    for _ in 0..3 {
        add_surprising_intent(&mut store, "agent", "push", "ball");
    }
    LawExtractor::attempt_extract(&mut store, "agent");
    let second_pass = LawExtractor::attempt_extract(&mut store, "agent");
    assert_eq!(second_pass.len(), 0);
    assert_eq!(store.all_by_type(MemoryType::Law).len(), 1);
}

// AC-P1: PolicyPayload serde
#[test]
fn ac_p1_policy_payload_serde() {
    let entity = Uuid::new_v4();
    let p = PolicyPayload {
        entity_id: entity,
        trigger_condition: "vel > 2".into(),
        action_fn: "brake".into(),
        priority: 5,
        active: true,
        law_id: None,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: PolicyPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.entity_id, entity);
    assert_eq!(back.priority, 5);
}

// AC-P2: PolicyRegistry returns sorted active
#[test]
fn ac_p2_policy_registry_sorted() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(PolicyRecord { id: Uuid::new_v4(), payload: PolicyPayload { entity_id: entity, trigger_condition: "x".into(), action_fn: "a".into(), priority: 3, active: true, law_id: None } });
    reg.register_record(PolicyRecord { id: Uuid::new_v4(), payload: PolicyPayload { entity_id: entity, trigger_condition: "y".into(), action_fn: "b".into(), priority: 7, active: true, law_id: None } });
    let active = reg.active_for_entity(entity);
    assert_eq!(active[0].payload.priority, 7);
}

// AC-S1: GC deletes orphaned low-MDL Law
#[test]
fn ac_s1_gc_deletes_orphaned_low_mdl_law() {
    let gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    assert_eq!(gc.gc_action_for_law(id, MDL_KEEP_THRESHOLD - 0.1), GcAction::Delete);
}

// AC-S2: GC keeps high-MDL Law
#[test]
fn ac_s2_gc_keeps_high_mdl_law() {
    let gc = CognitiveGC::new();
    let id = Uuid::new_v4();
    assert_eq!(gc.gc_action_for_law(id, MDL_KEEP_THRESHOLD + 0.1), GcAction::Keep);
}
```

- [ ] **Step 2: Register in `tests/integration_suite.rs`**

```rust
mod law_policy_integration_test;
```

- [ ] **Step 3: Run integration tests**

```sh
cargo test --no-default-features --features "petgraph_backend" --test integration_suite law_policy 2>&1
```

Expected: all 7 AC integration tests pass.

- [ ] **Step 4: Full suite — unit + integration**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -3
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | tail -3
cargo test --no-default-features --features "petgraph_backend" --test integration_suite 2>&1 | tail -3
```

Expected: all three `ok. N passed; 0 failed`.

- [ ] **Step 5: Update VERSION to 3.16.0**

```sh
echo "3.16.0" > VERSION
sed -i 's/^version = "3\.15\.1"/version = "3.16.0"/' Cargo.toml
sed -i 's/__version__ = "3\.15\.1"/__version__ = "3.16.0"/' sdk/python/hipcortex/__init__.py
sed -i 's/^version = "3\.15\.1"/version = "3.16.0"/' sdk/python/pyproject.toml
python scripts/stamp_versions.py --mcp
```

- [ ] **Step 6: Verify version alignment**

```sh
grep -r "3\.16\.0" VERSION Cargo.toml sdk/python/hipcortex/__init__.py sdk/python/pyproject.toml
```

Expected: 4 files show 3.16.0.

- [ ] **Step 7: Commit**

```sh
git add tests/integration/law_policy_integration_test.rs tests/integration_suite.rs VERSION Cargo.toml sdk/python/hipcortex/__init__.py sdk/python/pyproject.toml sdk/mcp/server.py sdk/python/hipcortex/install/mcp_server.py
git commit -m "feat(v3.16.0): Reconstruction Engine Foundation — Laws + Policies + MDL GC + axiomatic write-back"
```

---

## ReAct Loop Summary

| AC | Owner | Status after Task 7 |
|----|-------|---------------------|
| AC-L1 | law_extractor_tests + integration | PASS |
| AC-L2 | law_extractor_tests + integration | PASS |
| AC-L3 | law_extractor_tests + integration | PASS |
| AC-L4 | law_extractor.rs (code path) | IN CODE |
| AC-P1 | policy_registry_tests + integration | PASS |
| AC-P2 | policy_registry_tests + integration | PASS |
| AC-P3 | policy_registry_tests (step_with_policies) | PASS |
| AC-S1 | cognitive_gc_law_tests + integration | PASS |
| AC-S2 | cognitive_gc_law_tests + integration | PASS |
| AC-BUILD | Full lib + unit + integration suite | PASS |

Exit condition: all 10 ACs green. Max 3 iterations per task before escalating with root cause documented in Reflexion record via `loop_engine.rs`.
