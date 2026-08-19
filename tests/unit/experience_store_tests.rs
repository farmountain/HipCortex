use hipcortex::experience_store::ExperienceStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use serde_json::json;
use uuid::Uuid;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

#[test]
fn empty_store_returns_zero_tiers() {
    let store = make_store();
    let es = ExperienceStore::from_store(&store, "actor");
    assert_eq!(es.raw_count(), 0);
    assert_eq!(es.episode_count(), 0);
    assert_eq!(es.abstract_count(), 0);
}

#[test]
fn temporal_records_classified_as_raw() {
    let mut store = make_store();
    for i in 0..5 {
        store.add(MemoryRecord::new(
            MemoryType::Temporal,
            "actor".to_string(),
            "action".to_string(),
            format!("t{i}"),
            json!({}),
        )).unwrap();
    }
    let es = ExperienceStore::from_store(&store, "actor");
    assert_eq!(es.raw_count(), 5);
}

#[test]
fn skill_with_evidence_classified_as_episode() {
    let mut store = make_store();
    let src = MemoryRecord::new(MemoryType::Temporal, "actor".to_string(),
        "src".to_string(), "t0".to_string(), json!({}));
    let src_id = src.id;
    store.add(src).unwrap();
    let mut skill = MemoryRecord::new(MemoryType::Skill, "actor".to_string(),
        "induced".to_string(), "skill0".to_string(), json!({}));
    skill.evidence.push(src_id);
    store.add(skill).unwrap();
    let es = ExperienceStore::from_store(&store, "actor");
    assert_eq!(es.episode_count(), 1);
}

#[test]
fn consolidated_temporal_classified_as_abstract() {
    let mut store = make_store();
    let r = MemoryRecord::new(MemoryType::Temporal, "actor".to_string(),
        "consolidated".to_string(), "summary:run-1".to_string(), json!({}));
    store.add(r).unwrap();
    let es = ExperienceStore::from_store(&store, "actor");
    assert_eq!(es.abstract_count(), 1);
}

#[test]
fn reduction_ratio_correct() {
    let mut store = make_store();
    for i in 0..100 {
        store.add(MemoryRecord::new(
            MemoryType::Temporal, "actor".to_string(),
            "action".to_string(), format!("t{i}"), json!({}),
        )).unwrap();
    }
    for i in 0..5 {
        let mut s = MemoryRecord::new(MemoryType::Skill, "actor".to_string(),
            "sk".to_string(), format!("ep{i}"), json!({}));
        s.evidence.push(Uuid::new_v4());
        store.add(s).unwrap();
    }
    let es = ExperienceStore::from_store(&store, "actor");
    assert_eq!(es.raw_count(), 100);
    assert_eq!(es.episode_count(), 5);
}

#[test]
fn search_compressed_finds_matching_targets() {
    let mut store = make_store();
    let mut skill = MemoryRecord::new(MemoryType::Skill, "actor".to_string(),
        "sk".to_string(), "skill:login-flow".to_string(), json!({}));
    skill.evidence.push(Uuid::new_v4());
    store.add(skill).unwrap();
    let es = ExperienceStore::from_store(&store, "actor");
    let results = es.search_compressed(&store, "login");
    assert_eq!(results.len(), 1);
    assert!(results[0].target.contains("login"));
}
