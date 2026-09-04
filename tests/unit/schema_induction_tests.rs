use hipcortex::consolidation::{mine_causal_motifs, induce_skill_record};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::SkillPayload;

fn temporal_chain(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, actions: &[&str]) {
    let mut prev_id: Option<uuid::Uuid> = None;
    for action in actions {
        let mut rec = MemoryRecord::new(
            MemoryType::Temporal, "agent".into(), action.to_string(),
            format!("{}_result", action),
            serde_json::json!({ "thought": action }),
        );
        rec.derived_from = prev_id;
        prev_id = Some(rec.id);
        store.add(rec).unwrap();
    }
}

#[test]
fn induced_skill_preconditions_nonempty() {
    let mut store = MemoryStore::new_in_memory();
    // Build 2 identical chains so frequency >= 2
    temporal_chain(&mut store, &["observe", "reflect", "act"]);
    temporal_chain(&mut store, &["observe", "reflect", "act"]);

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(!motifs.is_empty(), "expected at least one motif");

    let skill = induce_skill_record(&motifs[0], "agent", &store);
    let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

    assert!(!p.preconditions.is_empty(), "preconditions must be non-empty: {:?}", p.preconditions);
}

#[test]
fn induced_skill_expected_outcomes_nonempty() {
    let mut store = MemoryStore::new_in_memory();
    temporal_chain(&mut store, &["observe", "reflect", "act"]);
    temporal_chain(&mut store, &["observe", "reflect", "act"]);

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(!motifs.is_empty());

    let skill = induce_skill_record(&motifs[0], "agent", &store);
    let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

    assert!(!p.expected_outcomes.is_empty(), "expected_outcomes must be non-empty: {:?}", p.expected_outcomes);
    // Must not be the old placeholder string
    assert!(
        !p.expected_outcomes[0].contains("pattern repeats"),
        "must not be old placeholder, got: {:?}", p.expected_outcomes
    );
}

#[test]
fn induced_skill_procedure_contains_action_sequence() {
    let mut store = MemoryStore::new_in_memory();
    temporal_chain(&mut store, &["observe", "reflect", "act"]);
    temporal_chain(&mut store, &["observe", "reflect", "act"]);

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(!motifs.is_empty());

    let skill = induce_skill_record(&motifs[0], "agent", &store);
    let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

    // Procedure must include the actions joined with →
    for action in &motifs[0].action_sequence {
        assert!(
            p.procedure.contains(action.as_str()),
            "procedure must contain '{}', got: {}", action, p.procedure
        );
    }
}

#[test]
fn preconditions_contain_first_action_or_target() {
    let mut store = MemoryStore::new_in_memory();
    temporal_chain(&mut store, &["observe", "reflect", "act"]);
    temporal_chain(&mut store, &["observe", "reflect", "act"]);

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(!motifs.is_empty());

    let skill = induce_skill_record(&motifs[0], "agent", &store);
    let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

    let precond_text = p.preconditions.join(" ");
    // Preconditions should mention the first action OR "requires"
    assert!(
        precond_text.contains("requires") || precond_text.contains("observe"),
        "preconditions must mention first action context, got: {:?}", p.preconditions
    );
}
