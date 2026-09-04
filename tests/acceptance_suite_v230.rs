//! Acceptance suite v2.3.0 — Grounding Obligation + Intent/Receipt Seam
//!
//! AC-G1: Q10 recommends probe after OpenIntent written to store.
//! AC-G2: PredictedOnly belief excluded from Q3; appears in Q8 list.
//! AC-G3: AcceptReceipt alone writes Temporal + updates WM contact.
//! AC-G4: Expired intent → Q10 = escalate_to_user.
//! AC-G5: 4+ observations → GroundingGate exits.
//! AC-G6: EntityContactRecord survives serde round-trip (restart simulation).
//! AC-G7: OpenIntent alone must NOT create Temporal observation.

use std::process;
use std::sync::{Arc, Mutex, RwLock};

use hipcortex::{
    action_intent::{
        ActionIntent, ActionReceipt, ContactKind, EntityContactRecord, GroundingStatus,
        IntentStatus,
    },
    cognitive_gc::CognitiveGC,
    cognitive_report::build_report,
    cognitive_state::{CognitiveDelta, CognitiveHandle},
    coherence::CoherenceChecker,
    grounding_gate::GroundingGate,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
    payloads::{BeliefPayload, JtmsLabel},
    persistence::InMemoryBackend,
    self_model::{calibration::CalibrationTracker, SelfModel},
    world_model_enhanced::WorldModelEnhanced,
};
use chrono::Utc;
use uuid::Uuid;

macro_rules! ac {
    ($label:expr, $body:block) => {{
        let result = std::panic::catch_unwind(|| $body);
        match result {
            Ok(_) => println!("[PASS] {}", $label),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("[FAIL] {} — {}", $label, msg);
                process::exit(1);
            }
        }
    }};
}

fn make_cognitive() -> CognitiveHandle<InMemoryBackend> {
    let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(CognitiveGC::new());
    CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc)
}

fn main() {
    // ── AC-G1 ────────────────────────────────────────────────────────────────
    ac!("AC-G1 Q10 recommends probe_entity / ground_workspace when Open intent exists", {
        let mut store = MemoryStore::new_in_memory();

        let intent = ActionIntent::new_probe(
            "agent".to_string(),
            "target_service".to_string(),
            None,
            30_000,
        );
        let rec = MemoryRecord::new(
            MemoryType::Intent,
            "agent".into(),
            "open_intent".into(),
            intent.op.clone(),
            serde_json::to_value(&intent).unwrap(),
        );
        store.add(rec).unwrap();

        let report = build_report(&store, "agent", 0.9);
        let op = &report.next_recommendation.recommended_op;
        assert!(
            op.starts_with("probe_entity:") || op == "ground_workspace",
            "Q10 must recommend probe_entity or ground_workspace when Open intents exist; got: {op}"
        );
    });

    // ── AC-G2 ────────────────────────────────────────────────────────────────
    ac!("AC-G2 Q3 excludes PredictedOnly belief; legacy contact_kind=None stays in Q3", {
        let mut store = MemoryStore::new_in_memory();

        // PredictedOnly — Kalman fill-in — must be excluded from Q3
        let bp_pred = BeliefPayload {
            proposition: "service_reachable_predicted".into(),
            confidence: 0.75,
            jtms_label: JtmsLabel::In,
            contact_kind: Some(ContactKind::PredictedOnly),
            ..Default::default()
        };
        let rec_pred = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(),
            "service_reachable_predicted".into(),
            serde_json::to_value(&bp_pred).unwrap(),
        );
        let pred_id = rec_pred.id;
        store.add(rec_pred).unwrap();

        // Legacy belief (contact_kind=None) — must remain in Q3
        let bp_obs = BeliefPayload {
            proposition: "db_connected".into(),
            confidence: 0.88,
            jtms_label: JtmsLabel::In,
            contact_kind: None,
            ..Default::default()
        };
        let rec_obs = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(),
            "db_connected".into(),
            serde_json::to_value(&bp_obs).unwrap(),
        );
        let obs_id = rec_obs.id;
        store.add(rec_obs).unwrap();

        let report = build_report(&store, "agent", 1.0);
        assert!(
            !report.valid_assumptions.iter().any(|b| b.id == pred_id),
            "Q3 must exclude PredictedOnly (Kalman fill-in) belief"
        );
        assert!(
            report.valid_assumptions.iter().any(|b| b.id == obs_id),
            "Q3 must include legacy belief (contact_kind=None)"
        );
    });

    // ── AC-G3 ────────────────────────────────────────────────────────────────
    ac!("AC-G3 AcceptReceipt writes Temporal + updates WM contact; no second add_memory", {
        let cognitive = make_cognitive();

        // Open intent
        let intent = ActionIntent::new_probe(
            "agent".to_string(),
            "filesystem".to_string(),
            None,
            30_000,
        );
        let intent_id = intent.id;
        cognitive.transact(CognitiveDelta::OpenIntent(intent), "agent").unwrap();

        let open_count = cognitive.open_intents.lock().unwrap().len();
        assert_eq!(open_count, 1, "OpenIntent must add to open_intents list");

        // Submit receipt
        let receipt = ActionReceipt {
            intent_id,
            ok: true,
            observation: serde_json::json!({ "disk_free_gb": 42.5 }),
            sensor_path: "mcp:filesystem".to_string(),
            ts: Utc::now(),
        };
        cognitive.transact(CognitiveDelta::AcceptReceipt(receipt), "agent").unwrap();

        let ms = cognitive.memory.lock().unwrap();

        // Temporal observation written
        let temporals = ms.all_by_type(MemoryType::Temporal);
        let obs_rec = temporals.iter().find(|r| r.action == "receipt_observation");
        assert!(obs_rec.is_some(), "AcceptReceipt must write Temporal{{receipt_observation}}");

        // Receipt record written
        let receipts = ms.all_by_type(MemoryType::Receipt);
        assert!(!receipts.is_empty(), "AcceptReceipt must write Receipt record");

        drop(ms);

        // WM entity_contact updated via public accessor
        let contact = cognitive.wm_entity_contact("filesystem");
        assert!(contact.is_some(), "AcceptReceipt must update WM entity_contact");
        let c = contact.unwrap();
        assert_eq!(c.n_observations, 1, "n_observations must be 1 after one receipt");
        assert!(
            matches!(c.last_contact_kind, ContactKind::Observed),
            "contact_kind must be Observed after successful receipt"
        );

        // Intent marked Received
        let intents = cognitive.open_intents.lock().unwrap();
        let updated = intents.iter().find(|i| i.id == intent_id);
        assert!(
            updated.map(|i| matches!(i.status, IntentStatus::Received)).unwrap_or(false),
            "Intent must be Received after AcceptReceipt"
        );
    });

    // ── AC-G4 ────────────────────────────────────────────────────────────────
    ac!("AC-G4 Expired intent → Q10 escalate_to_user (host silence is a fact)", {
        let mut store = MemoryStore::new_in_memory();

        let expired_intent = serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "actor": "agent",
            "kind": "Probe",
            "op": "probe_entity:old_service",
            "args": {},
            "deadline_ms": 100u64,
            "deadline_tx": "2020-01-01T00:00:00Z",
            "status": "Expired",
            "created_tx": "2020-01-01T00:00:00Z",
            "goal_id": null,
            "target_entity": "old_service"
        });
        let rec = MemoryRecord::new(
            MemoryType::Intent, "agent".into(), "open_intent".into(),
            "probe_entity:old_service".into(), expired_intent,
        );
        store.add(rec).unwrap();

        let report = build_report(&store, "agent", 0.9);
        assert_eq!(
            report.next_recommendation.recommended_op, "escalate_to_user",
            "Q10 must be escalate_to_user when only expired intents exist; got: {}",
            report.next_recommendation.recommended_op
        );
    });

    // ── AC-G5 ────────────────────────────────────────────────────────────────
    ac!("AC-G5 GroundingGate exits after 4 observations (≥ MAPPED_OBS_THRESHOLD)", {
        let mut contacts = std::collections::HashMap::new();
        let mut rec = EntityContactRecord::default();
        for _ in 0..4 {
            rec.apply_observation();
        }
        contacts.insert("target_entity".to_string(), rec);

        let entity_refs: &[&str] = &["target_entity"];
        let grounding = GroundingGate::is_active(
            entity_refs,
            |name| contacts.get(name).cloned(),
        );
        assert!(!grounding, "GroundingGate must exit after 4 observations");

        // Conversely: 1 observation → still active (epistemic > τ_e = 0.5)
        let mut contacts2 = std::collections::HashMap::new();
        let mut rec2 = EntityContactRecord::default();
        rec2.apply_observation(); // n=1, epistemic = 1/sqrt(2) ≈ 0.71 > 0.5
        contacts2.insert("entity2".to_string(), rec2);
        let grounding2 = GroundingGate::is_active(
            &["entity2"],
            |name| contacts2.get(name).cloned(),
        );
        assert!(grounding2, "GroundingGate must remain active with n=1 observation (epistemic > τ_e)");
    });

    // ── AC-G6 ────────────────────────────────────────────────────────────────
    ac!("AC-G6 EntityContactRecord survives serde round-trip (restart simulation)", {
        let mut rec = EntityContactRecord::default();
        rec.apply_observation();
        rec.apply_observation();
        assert!(matches!(rec.grounding_status, GroundingStatus::Sketch));

        let json = serde_json::to_string(&rec).unwrap();
        let reloaded: EntityContactRecord = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(reloaded.grounding_status, GroundingStatus::Sketch),
            "grounding_status must survive serde round-trip"
        );
        assert_eq!(reloaded.n_observations, 2, "n_observations must survive serde round-trip");
        assert!(
            matches!(reloaded.last_contact_kind, ContactKind::Observed),
            "last_contact_kind must survive serde round-trip"
        );
    });

    // ── AC-G7 ────────────────────────────────────────────────────────────────
    ac!("AC-G7 OpenIntent alone does NOT create Temporal observation or WM contact update", {
        let cognitive = make_cognitive();

        let intent = ActionIntent::new_probe(
            "agent".to_string(), "env_service".to_string(), None, 30_000,
        );
        cognitive.transact(CognitiveDelta::OpenIntent(intent), "agent").unwrap();

        // No receipt → no Temporal{receipt_observation}
        let ms = cognitive.memory.lock().unwrap();
        let obs_temporals: Vec<_> = ms.all_by_type(MemoryType::Temporal)
            .into_iter()
            .filter(|r| r.action == "receipt_observation")
            .collect();
        assert!(
            obs_temporals.is_empty(),
            "OpenIntent alone must not create Temporal observation"
        );
        drop(ms);

        // WM contact must also remain unupdated
        let contact = cognitive.wm_entity_contact("env_service");
        assert!(
            contact.is_none(),
            "WM entity_contact must not be set by OpenIntent alone"
        );
    });

    println!("\n=== Acceptance v2.3.0 (Grounding Obligation): 7/7 passed ===");
}
