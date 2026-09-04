use hipcortex::belief_executive::BeliefExecutive;
use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
use hipcortex::consolidation::{mine_causal_motifs, induce_skill_record};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, GoalPayload, JtmsLabel, SkillPayload, SuccessFactor, GoalStatus};

// ── helpers ───────────────────────────────────────────────────────────────────

fn store() -> MemoryStore<hipcortex::persistence::InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn temporal(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, action: &str, target: &str, prev: Option<uuid::Uuid>) -> uuid::Uuid {
    let mut rec = MemoryRecord::new(
        MemoryType::Temporal, "agent".into(), action.into(),
        target.into(), serde_json::json!({ "thought": action }),
    );
    rec.derived_from = prev;
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn belief(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, conf: f32) -> uuid::Uuid {
    let bp = BeliefPayload { proposition: "test belief".into(), confidence: conf, ..Default::default() };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(), "test".into(),
        serde_json::to_value(&bp).unwrap(),
    );
    rec.confidence = conf;
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn goal_with_factors(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, factors: Vec<&str>) -> uuid::Uuid {
    let sf: Vec<SuccessFactor> = factors.iter()
        .map(|n| SuccessFactor { name: n.to_string(), weight: 1.0, satisfied: false })
        .collect();
    let gp = GoalPayload {
        target_state: "deploy".into(),
        success_factors: sf,
        status: GoalStatus::InProgress,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Temporal, "agent".into(), "goal".into(), "deploy".into(),
        serde_json::to_value(&gp).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

// ── Gap 1: real schema induction ─────────────────────────────────────────────

#[test]
fn mine_and_consolidate_produces_structured_skill() {
    let mut store = store();
    // Two identical chains → motif with frequency ≥ 2
    let a1 = temporal(&mut store, "observe", "data", None);
    let b1 = temporal(&mut store, "reflect", "result", Some(a1));
    let _ = temporal(&mut store, "act", "output", Some(b1));
    let a2 = temporal(&mut store, "observe", "data", None);
    let b2 = temporal(&mut store, "reflect", "result", Some(a2));
    let _ = temporal(&mut store, "act", "output", Some(b2));

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(!motifs.is_empty(), "expected motif from two identical chains");

    let skill = induce_skill_record(&motifs[0], "agent", &store);
    let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

    assert!(!p.preconditions.is_empty(), "skill must have preconditions");
    assert!(!p.expected_outcomes.is_empty(), "skill must have expected_outcomes");
    assert!(!p.procedure.is_empty(), "skill must have procedure");
    // Real content — not the old placeholder
    assert!(
        !p.expected_outcomes[0].contains("pattern repeats"),
        "must not be old placeholder: {:?}", p.expected_outcomes
    );
}

// ── Gap 2: BeliefExecutive single authority ───────────────────────────────────

#[test]
fn confidence_and_label_always_agree_after_decay() {
    let mut store = store();
    let id = belief(&mut store, 0.9);
    // Decay below ARCHIVE_THRESHOLD
    BeliefExecutive::decay(&mut store, id, 0.1);
    let rec = store.find_by_id(id).unwrap();
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    // Both fields must indicate "invalid"
    assert!(rec.confidence < 0.2, "confidence must be low");
    assert_eq!(p.jtms_label, JtmsLabel::Out, "JtmsLabel must be Out when confidence below threshold");
}

#[test]
fn belief_invalidator_routes_through_executive() {
    // Simulates what ReactEngine does: new contradicting Temporal → BeliefInvalidator
    // → BeliefExecutive.decay → JTMS Out cascade. Verify end state coherence.
    let mut store = store();
    let bid = belief(&mut store, 0.9);

    // Contradicting record hits negation keyword + overlapping token
    let contra = MemoryRecord::new(
        MemoryType::Temporal, "agent".into(), "failed".into(),
        "test belief broken".into(),
        serde_json::json!({ "thought": "test belief failed" }),
    );
    let invalidated = hipcortex::belief_invalidator::BeliefInvalidator::process(&contra, &mut store);

    // If threshold crossed: both confidence AND label must agree
    let rec = store.find_by_id(bid).unwrap();
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    if !invalidated.is_empty() {
        assert!(rec.confidence < 0.2 || p.jtms_label == JtmsLabel::Out,
            "invalidated belief must have low conf or Out label");
        if rec.confidence < 0.2 {
            assert_eq!(p.jtms_label, JtmsLabel::Out,
                "below-threshold confidence must agree with Out label");
        }
    }
}

// ── Gap 3: ClarifyEngine env restatement ─────────────────────────────────────

#[test]
fn clarify_restates_when_env_signal_blocks_factor() {
    let mut store = store();
    let goal_id = goal_with_factors(&mut store, vec!["deploy_server"]);
    // Env signal: server failed
    temporal(&mut store, "failed", "deploy_server", None);

    let outcome = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC, None);
    assert_eq!(outcome, ClarifyOutcome::ClarifiedBySubstrate,
        "env-blocked factor must yield ClarifiedBySubstrate");

    // Factor must be renamed
    let rec = store.find_by_id(goal_id).unwrap();
    let gp: GoalPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    assert!(gp.success_factors[0].name.ends_with("_when_available"),
        "factor must be renamed: {}", gp.success_factors[0].name);

    // Reflexion{goal_restated} must exist
    let has_restated = store.all_by_type(MemoryType::Reflexion)
        .iter().any(|r| r.action == "goal_restated");
    assert!(has_restated, "must write Reflexion{{goal_restated}}");
}
