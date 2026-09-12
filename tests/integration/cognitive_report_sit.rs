#![cfg(feature = "web-server")]
//! SIT: `/v1/cognitive/report` actor scoping (WP3 / H4 / D9).
//!
//! H4 was "the report ignores `actor`": Q2 (learned beliefs), Q3 (valid
//! assumptions), Q4/Q5 (decisions), Q7 (abstractions) and Q8 (predicted-only
//! exclusions) all read `store.all_by_type(...)` with **no** actor filter, so a
//! caller asking about actor A was handed actor B's knowledge. The handover's
//! phrasing is the reason this file exists: *"Worse than empty: it looks like it
//! works."*
//!
//! Two independent failure modes are guarded here:
//!
//! 1. **Leakage** — supplying an actor must not surface another actor's records.
//!    A fabricated actor must yield 0 beliefs and 0 decisions (spec §WP3).
//! 2. **Silent defaulting** — omitting `actor` must not be answered with a magic
//!    actor's data. It returns `actor_scoped: false` and every list empty
//!    (spec D9).
//!
//! Run: cargo test --no-default-features --features "petgraph_backend,web-server" \
//!           --test integration_suite cognitive_report

use hipcortex::{
    archive_store::ArchiveStore,
    aureus_bridge::AureusBridge,
    cognitive_gc::CognitiveGC,
    cognitive_state::CognitiveHandle,
    coherence::CoherenceChecker,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
    payloads::{BeliefPayload, GoalPayload, JtmsLabel},
    self_model::{calibration::CalibrationTracker, SelfModel},
    substrate_daemon::SubstrateDaemon,
    symbolic_store::SymbolicStore,
    topological_memory::CausalTopoGraph,
    web_server::{build_app, AppState},
    workspace::WorkspaceRegistry,
    world_model_enhanced::WorldModelEnhanced,
    InMemoryBackend,
};
use std::sync::{Arc, Mutex, RwLock};

type TestState = AppState<InMemoryBackend>;

fn make_test_state() -> TestState {
    let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let coherence = Arc::new(CoherenceChecker::new());
    let calibration = Arc::new(CalibrationTracker::new());
    let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let self_model = Arc::new(SelfModel::new());
    let cognitive = Arc::new(CognitiveHandle::new(
        Arc::clone(&memory_store),
        Arc::clone(&world_model),
        Arc::clone(&self_model),
        None,
        Arc::clone(&coherence),
        Arc::clone(&calibration),
        Arc::new(CognitiveGC::new()),
    ));
    let archive_path =
        std::env::temp_dir().join(format!("hipcortex-report-test-{}.jsonl", uuid::Uuid::new_v4()));
    AppState {
        memory_store,
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model,
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model,
        coherence,
        topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
        archive_store: Arc::new(Mutex::new(ArchiveStore::new(archive_path))),
        tx_log: None,
        calibration,
        cognitive,
        forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
        daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
        workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
        passive_capture_enabled: true,
    }
}

/// Spin up a real local server on a random port. Returns (base_url, shutdown_handle).
async fn start_test_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let app = build_app(state);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap_or_default();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    (format!("http://{}", addr), handle)
}

/// Seed one Belief, one Decision and one Goal for `actor`, written **through the
/// store** so the SHA-256 integrity hash and audit chain stay consistent
/// (persistence rule: never hand-build a `MemoryRecord` outside `MemoryStore`).
///
/// The JTMS label is set explicitly: `JtmsLabel::default()` is `Unknown`, and
/// Q2 ("learned") only admits `In` beliefs above 0.3, so leaving it default
/// would make the "own records are visible" assertion pass vacuously.
fn seed(store: &Arc<Mutex<MemoryStore<InMemoryBackend>>>, actor: &str) -> (uuid::Uuid, uuid::Uuid) {
    let mut guard = store.lock().unwrap();

    let proposition = format!("{actor} is reachable on the internal mesh");
    let belief_payload = BeliefPayload {
        proposition: proposition.clone(),
        justification: "observed on the mesh".to_string(),
        confidence: 0.9,
        jtms_label: JtmsLabel::In,
        ..Default::default()
    };
    let belief = MemoryRecord::new(
        MemoryType::Belief,
        actor.to_string(),
        "asserts".to_string(),
        proposition,
        serde_json::to_value(&belief_payload).unwrap(),
    );
    let belief_id = belief.id;
    guard.add(belief).unwrap();

    let decision = MemoryRecord::new(
        MemoryType::Decision,
        actor.to_string(),
        "chose".to_string(),
        "plan-alpha".to_string(),
        serde_json::json!({
            "option_chosen": "plan-alpha",
            "rationale_chain": ["cheapest option that satisfies the goal"],
            "confidence": 0.8
        }),
    );
    let decision_id = decision.id;
    guard.add(decision).unwrap();

    let goal_payload = GoalPayload {
        target_state: format!("{actor} ships the release"),
        urgency: 0.9,
        ..Default::default()
    };
    let goal = MemoryRecord::new(
        MemoryType::Goal,
        actor.to_string(),
        "pursues".to_string(),
        "ship the release".to_string(),
        serde_json::to_value(&goal_payload).unwrap(),
    );
    guard.add(goal).unwrap();

    (belief_id, decision_id)
}

async fn report_for(base: &str, query: &str) -> serde_json::Value {
    reqwest::Client::new()
        .get(format!("{}/v1/cognitive/report{}", base, query))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

// ── AC / WP3: fabricated actor returns 0 beliefs and 0 decisions ─────────────

#[tokio::test]
async fn h4_fabricated_actor_reports_nothing() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "?actor=no-such-actor-exists").await;
    srv.abort();

    assert_eq!(body["actor_scoped"], true, "an explicitly named actor is scoped: {body}");
    // H6/WP8: `learned_beliefs` is a count; the array is `learned_beliefs_detail`.
    assert_eq!(
        body["learned_beliefs"].as_u64(),
        Some(0),
        "a fabricated actor must not inherit another actor's beliefs: {body}"
    );
    assert_eq!(
        body["learned_beliefs_detail"].as_array().map(|a| a.len()),
        Some(0),
        "a fabricated actor must not inherit another actor's belief detail: {body}"
    );
    assert_eq!(
        body["recent_decisions"].as_array().map(|a| a.len()),
        Some(0),
        "a fabricated actor must not inherit another actor's decisions: {body}"
    );
    assert_eq!(
        body["active_goals"].as_array().map(|a| a.len()),
        Some(0),
        "a fabricated actor must not inherit another actor's goals: {body}"
    );
}

// ── AC / WP3: the real actor still sees its own records (guard did not break
//    the happy path — a report that always returns nothing would also pass the
//    test above, which is why this companion test is mandatory) ──────────────

#[tokio::test]
async fn h4_named_actor_sees_its_own_records() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "?actor=alice").await;
    srv.abort();

    assert_eq!(body["actor_scoped"], true);
    assert_eq!(body["actor"], "alice");
    assert_eq!(
        body["learned_beliefs"].as_u64(),
        Some(1),
        "alice's own belief must be visible to alice: {body}"
    );
    assert_eq!(
        body["learned_beliefs_detail"].as_array().map(|a| a.len()),
        Some(1),
        "alice's own belief must be visible to alice in detail: {body}"
    );
    assert_eq!(
        body["active_goals"].as_array().map(|a| a.len()),
        Some(1),
        "alice's own goal must be visible to alice: {body}"
    );
    assert_eq!(
        body["recent_decisions"].as_array().map(|a| a.len()),
        Some(1),
        "alice's own decision must be visible to alice: {body}"
    );
}

// ── AC / WP3: cross-actor isolation is bidirectional ────────────────────────

#[tokio::test]
async fn h4_two_actors_do_not_see_each_other() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    seed(&state.memory_store, "bob");
    let (base, srv) = start_test_server(state).await;

    let alice = report_for(&base, "?actor=alice").await;
    let bob = report_for(&base, "?actor=bob").await;
    srv.abort();

    for (who, body) in [("alice", &alice), ("bob", &bob)] {
        assert_eq!(
            body["learned_beliefs"].as_u64(),
            Some(1),
            "{who} must see exactly its own single belief, not both: {body}"
        );
    }

    // The propositions must differ — one belief each, each their own.
    let alice_prop = alice["learned_beliefs_detail"][0]["proposition"].as_str().unwrap_or_default();
    let bob_prop = bob["learned_beliefs_detail"][0]["proposition"].as_str().unwrap_or_default();
    assert!(alice_prop.contains("alice"), "alice's report leaked bob's belief: {alice_prop}");
    assert!(bob_prop.contains("bob"), "bob's report leaked alice's belief: {bob_prop}");
}

// ── AC / D9: omitted actor ⇒ explicit, empty, `actor_scoped: false` ─────────

#[tokio::test]
async fn d9_omitted_actor_is_explicitly_unscoped() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "").await;
    srv.abort();

    assert_eq!(
        body["actor_scoped"], false,
        "omitting `actor` must be visible to the client, not silently guessed: {body}"
    );
    // D9 is explicit that the alternative — returning a default actor's data —
    // is the bug. Every list must therefore be empty.
    for key in [
        "active_goals",
        "valid_assumptions",
        "recent_decisions",
        "recent_failures",
        "authorized_actions",
    ] {
        assert_eq!(
            body[key].as_array().map(|a| a.len()),
            Some(0),
            "unscoped report must not carry data in `{key}`: {body}"
        );
    }
    // H6/WP8: the two count keys must be zero and their detail arrays empty.
    for (count_key, detail_key) in [
        ("learned_beliefs", "learned_beliefs_detail"),
        ("emergent_abstractions", "emergent_abstractions_detail"),
    ] {
        assert_eq!(
            body[count_key].as_u64(),
            Some(0),
            "unscoped report must not carry a `{count_key}` count: {body}"
        );
        assert_eq!(
            body[detail_key].as_array().map(|a| a.len()),
            Some(0),
            "unscoped report must not carry data in `{detail_key}`: {body}"
        );
    }
    assert_eq!(
        body["open_uncertainties"]["uncertain_beliefs"].as_array().map(|a| a.len()),
        Some(0),
        "unscoped report must not carry uncertainties: {body}"
    );
    assert_eq!(
        body["next_recommendation"]["recommended_op"], "supply_actor",
        "the unscoped report must tell the caller how to fix the request: {body}"
    );
}

// ── AC / D9: an empty-string actor is the same as an omitted one ────────────

#[tokio::test]
async fn d9_blank_actor_is_treated_as_omitted() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "?actor=").await;
    srv.abort();

    assert_eq!(
        body["actor_scoped"], false,
        "`?actor=` names nobody, so it must not silently select an actor: {body}"
    );
}

// ── Regression: the report must not be persisted as a side effect ───────────
//
// The handler takes `cog.memory.lock()`. A read path that mutated the store
// would inflate Q2 on the next call; asserting idempotence catches that.

#[tokio::test]
async fn h4_report_is_read_only() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let first = report_for(&base, "?actor=alice").await;
    let second = report_for(&base, "?actor=alice").await;
    srv.abort();

    assert_eq!(
        first["learned_beliefs"].as_u64(),
        second["learned_beliefs"].as_u64(),
        "asking for the report twice must not change it: {first} vs {second}"
    );
    assert_eq!(first["learned_beliefs"].as_u64(), Some(1));
}

// ── H6 / WP8: the count is the question's answer; the array is the evidence ──
//
// H6 filed `learned_beliefs` / `emergent_abstractions` as arrays: the cost of
// asking Q2 or Q7 grew with how much the system had learned, for a question
// whose answer is a cardinality. The wire shape is now count + `*_detail`.
// These assertions pin the invariant that makes the split safe — the count must
// always equal the detail length, so a client that trusts the number and a
// client that reads the array can never be told different things.

#[tokio::test]
async fn h6_q2_and_q7_expose_a_count_beside_their_detail() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "?actor=alice").await;
    srv.abort();

    for (count_key, detail_key) in [
        ("learned_beliefs", "learned_beliefs_detail"),
        ("emergent_abstractions", "emergent_abstractions_detail"),
    ] {
        let count = body[count_key].as_u64().unwrap_or_else(|| {
            panic!("`{count_key}` must be a number (H6/WP8): {body}")
        });
        let detail_len = body[detail_key]
            .as_array()
            .unwrap_or_else(|| panic!("`{detail_key}` must be an array (H6/WP8): {body}"))
            .len();
        assert_eq!(
            count as usize, detail_len,
            "`{count_key}` ({count}) disagrees with `{detail_key}` ({detail_len}) — \
             a client trusting the count and a client reading the detail would be \
             told different things"
        );
    }

    // The seed writes one In-labelled belief, so the count must be exactly 1 —
    // a report that simply reports 0 for everything would satisfy the equality
    // above while being useless.
    assert_eq!(body["learned_beliefs"].as_u64(), Some(1), "expected alice's belief: {body}");
    assert_eq!(
        body["learned_beliefs_detail"].as_array().map(|a| a.len()),
        Some(1),
        "`learned_beliefs_detail` must still carry the belief itself: {body}"
    );
}

/// Both counts must be plain JSON numbers, not strings or arrays. A consumer
/// doing arithmetic on the value is the whole point of the change.
#[tokio::test]
async fn h6_counts_are_numbers_not_arrays() {
    let state = make_test_state();
    seed(&state.memory_store, "alice");
    let (base, srv) = start_test_server(state).await;

    let body = report_for(&base, "?actor=alice").await;
    srv.abort();

    for key in ["learned_beliefs", "emergent_abstractions"] {
        assert!(
            body[key].is_number(),
            "`{key}` must be a JSON number after H6/WP8, got {}: {body}",
            body[key]
        );
    }
}
