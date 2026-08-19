// Property and unit tests for mine_and_consolidate consolidation invariants.
//
// Tests verify:
// 1. Causal chains produce Skill induction (motif mining works).
// 2. Induced Skill records carry evidence links (provenance preserved).
// 3. Source records are archived after consolidation.
// 4. Property: repeated chains always produce skills when frequency >= min_freq.
//
// Note: 90% reduction target (AC-4 full) requires Sub-spec 1 ExperienceStore.
// Tests here verify the mining layer. See bottom for the full AC-4 TODO.

use hipcortex::consolidation::mine_and_consolidate;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::InMemoryBackend;
use proptest::prelude::*;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// Helper: build a causal chain of Temporal records linked via derived_from
// ============================================================================

fn build_chain(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    action: &str,
    length: usize,
) {
    let mut prev_id: Option<Uuid> = None;
    for step in 0..length {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            actor.to_string(),
            action.to_string(),
            format!("target-{step}"),
            json!({}),
        );
        r.derived_from = prev_id;
        prev_id = Some(r.id);
        store.add(r).expect("add record");
    }
}

// ============================================================================
// Unit test 1: Causal chains produce Skill induction
//
// 3 identical action chains x 10 records each -> motif frequency = 3.
// mine_and_consolidate(store, None, 3, actor) must induce >= 1 Skill.
// ============================================================================

#[test]
fn consolidation_induces_skills_from_causal_chains() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "consolidation-test";
    let action = "repeated-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 10);
    }

    let report = mine_and_consolidate(&mut store, None, 3, actor)
        .expect("mine_and_consolidate failed");

    assert!(
        report.skills_induced > 0,
        "expected >= 1 Skill induced from 3 identical causal chains, got skills_induced={}",
        report.skills_induced
    );
}

// ============================================================================
// Unit test 2: Induced Skills carry evidence links (provenance preserved)
// ============================================================================

#[test]
fn induced_skills_carry_evidence_links() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "evidence-test";
    let action = "evidence-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 10);
    }

    mine_and_consolidate(&mut store, None, 3, actor).expect("mine_and_consolidate failed");

    let skill_records = store.all_by_type(MemoryType::Skill);
    assert!(
        !skill_records.is_empty(),
        "no Skill records found after consolidation"
    );
    for skill in &skill_records {
        assert!(
            !skill.evidence.is_empty(),
            "Skill {} has empty evidence -- provenance lost during consolidation",
            skill.id
        );
    }
}

// ============================================================================
// Unit test 3: Source records archived after consolidation
// ============================================================================

#[test]
fn consolidation_archives_source_records() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "archive-test";
    let action = "archive-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 8);
    }

    let report = mine_and_consolidate(&mut store, None, 3, actor)
        .expect("mine_and_consolidate failed");

    assert!(
        !report.source_ids_archived.is_empty(),
        "expected source records archived, got source_ids_archived=[]"
    );
}

// ============================================================================
// Proptest: N chains x M records -> skills_induced > 0 when frequency >= min_freq
// ============================================================================

proptest! {
    #[test]
    fn consolidation_produces_skills_from_repeated_chains(
        chain_len in 3usize..12,
        n_chains in 3usize..6
    ) {
        let mut store = MemoryStore::new_in_memory();
        let actor = "prop-test-actor";
        let action = "prop-repeated-action";

        for _ in 0..n_chains {
            build_chain(&mut store, actor, action, chain_len);
        }

        let report = mine_and_consolidate(&mut store, None, n_chains, actor)
            .expect("mine_and_consolidate failed");

        prop_assert!(
            report.skills_induced > 0,
            "expected skills from {} identical chains of length {}, got skills_induced={}",
            n_chains,
            chain_len,
            report.skills_induced
        );
    }
}

// ============================================================================
// TODO(sub-spec-1): Full 90% reduction assertion -- activate after ExperienceStore ships
//
// #[test]
// fn experience_store_90_percent_reduction() {
//     // Insert 1000 Temporal records across 10 causal chains (100 per chain).
//     // Call AutoConsolidate delta via CognitiveHandle (Sub-spec 1 API).
//     // Assert: hot store record_count() <= 100 (>= 90% reduction).
//     // Assert: all source_ids_archived records reachable via evidence links
//     //         from remaining Skill/Belief records.
// }
// ============================================================================
