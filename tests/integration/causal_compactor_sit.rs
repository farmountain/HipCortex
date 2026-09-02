// SIT: causal motif mining induces Skills from recurring derived_from chains (AC-2).
//
// Writes 30 synthetic ReAct cycles (observe → reflect → act) where each record
// links to the previous via derived_from. mine_and_consolidate must:
//   - find the recurring pattern
//   - induce ≥ 1 Skill record with evidence links
//   - induce ≥ 1 Belief record
//   - archive the source episodes from hot store

use hipcortex::consolidation::mine_and_consolidate;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use uuid::Uuid;

fn write_chain(store: &mut MemoryStore<hipcortex::InMemoryBackend>, actor: &str) {
    let mut prev: Option<Uuid> = None;
    for action in ["observe", "reflect", "act"] {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            actor.to_string(),
            action.to_string(),
            format!("{action}-target"),
            serde_json::json!({}),
        );
        r.derived_from = prev;
        prev = Some(r.id);
        store.add(r).expect("store.add");
    }
}

#[test]
fn causal_motif_mining_induces_skills() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "causal-compactor-sit";

    // 30 identical ReAct chains — well above min_frequency=3
    for _ in 0..30 {
        write_chain(&mut store, actor);
    }

    let before_count = store.record_count();
    let report = mine_and_consolidate(&mut store, None, None, None, 3, actor)
        .expect("mine_and_consolidate must succeed");

    assert!(
        report.skills_induced >= 1,
        "must induce at least one Skill; got motifs={} skills={}",
        report.motifs_found,
        report.skills_induced,
    );
    assert!(
        report.beliefs_induced >= 1,
        "must induce at least one Belief; got {}",
        report.beliefs_induced,
    );
    assert!(
        !report.source_ids_archived.is_empty(),
        "source episodes must be archived from hot store",
    );

    // Hot store must shrink (source episodes removed, skills+beliefs added — net negative)
    let after_count = store.record_count();
    assert!(
        after_count < before_count,
        "hot store must shrink after compaction: before={before_count} after={after_count}",
    );

    // Induced Skill records must carry evidence links back to source episodes
    let skills: Vec<_> = store
        .all_by_type(MemoryType::Skill)
        .into_iter()
        .filter(|r| r.actor == actor)
        .collect();
    assert!(!skills.is_empty(), "Skill records must be present in hot store");
    for skill in &skills {
        assert!(
            !skill.evidence.is_empty(),
            "Skill {} must have evidence links",
            skill.id,
        );
    }
}
