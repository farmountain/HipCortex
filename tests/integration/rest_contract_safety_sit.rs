/// SIT: REST write-path contract safety.
///
/// Two independent silent-failure defects, both closed by the remediation spec:
///
/// * WP6 / H3 — `record_type` was coerced to `Temporal` on any unrecognised
///   input, so a typo turned a belief into an episode with no error returned.
/// * WP2 / H9 — `DELETE /memory/forget/:actor` was unactionable for compliance:
///   it reported a count but never disclosed *which* records it destroyed.
/// * WP6 / H3 (read path) — `GET /memory/query` kept its own five-name,
///   case-sensitive `record_type` list, so it rejected records the write path
///   had accepted moments earlier (`Goal`, `Semantic`). One vocabulary now.
///
/// And one spurious refusal, found while wiring the v0.4.0 contract suite into CI:
///
/// * `POST /memory/link` classified its two record UUIDs instead of any content,
///   and the US-phone pattern in the PII set reads the `NNNNNN-NNNN` straddle of a
///   UUID hyphen as a phone number — so roughly one link in twenty came back 403 at
///   random, on identifiers the caller neither chooses nor can re-roll.
///
/// Uses a real TCP server + reqwest to avoid axum-test / axum-0.6 version skew.
#[cfg(feature = "web-server")]
mod tests {
    use hipcortex::{
        archive_store::ArchiveStore,
        aureus_bridge::AureusBridge,
        cognitive_gc::CognitiveGC,
        cognitive_state::CognitiveHandle,
        coherence::CoherenceChecker,
        memory_record::{MemoryRecord, MemoryType},
        memory_store::MemoryStore,
        persistence::InMemoryBackend,
        self_model::{calibration::CalibrationTracker, SelfModel},
        substrate_daemon::SubstrateDaemon,
        symbolic_store::SymbolicStore,
        topological_memory::CausalTopoGraph,
        web_server::{build_app, AppState},
        workspace::WorkspaceRegistry,
        world_model_enhanced::WorldModelEnhanced,
    };
    use std::sync::{Arc, Mutex, RwLock};

    type Store = Arc<Mutex<MemoryStore<InMemoryBackend>>>;
    type TestState = AppState<InMemoryBackend>;

    fn make_test_state() -> (TestState, Store) {
        let memory_store: Store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let store_ref = memory_store.clone();
        let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
        let self_model = Arc::new(SelfModel::new());
        let coherence = Arc::new(CoherenceChecker::new());
        let calibration = Arc::new(CalibrationTracker::new());
        let cognitive = Arc::new(CognitiveHandle::new(
            Arc::clone(&memory_store),
            Arc::clone(&world_model),
            Arc::clone(&self_model),
            None,
            Arc::clone(&coherence),
            Arc::clone(&calibration),
            Arc::new(CognitiveGC::new()),
        ));

        let state = AppState {
            memory_store,
            symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
            world_model,
            aureus: Arc::new(Mutex::new(AureusBridge::new())),
            self_model,
            coherence,
            topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
            archive_store: Arc::new(Mutex::new(ArchiveStore::new_in_memory())),
            tx_log: None,
            calibration,
            cognitive,
            forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: false,
        };
        (state, store_ref)
    }

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
        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    // ── H3: record_type must never be silently coerced ────────────────────────

    // AC: an unknown record_type is a 400 naming the offending value.
    #[tokio::test]
    async fn unknown_record_type_is_rejected_not_coerced() {
        let (state, store) = make_test_state();
        let (base, _srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "typo-author",
                "action": "believed",
                "target": "the sky is green",
                "record_type": "Belif"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 400, "a typo must not be accepted");
        let v: serde_json::Value = resp.json().await.unwrap();
        assert!(
            v["error"].as_str().unwrap_or_default().contains("Belif"),
            "error must name the offending value, got {}",
            v["error"]
        );
        let valid = v["warning"]["valid_record_types"].as_array().unwrap();
        assert!(valid.iter().any(|x| x == "Belief"), "must publish the valid set");
        assert!(
            store.lock().unwrap().find_by_actor("typo-author").is_empty(),
            "a rejected record must not be stored"
        );
    }

    // AC: documented aliases keep working (no breaking change for existing clients).
    #[tokio::test]
    async fn documented_aliases_still_resolve() {
        let (state, store) = make_test_state();
        let (base, _srv) = start_test_server(state).await;

        for (alias, expected) in [
            ("short_term", MemoryType::Temporal),
            ("semantic", MemoryType::Symbolic),
            ("Reflexion", MemoryType::Reflexion),
            ("goal", MemoryType::Goal),
            ("receipt", MemoryType::Receipt),
        ] {
            let resp = reqwest::Client::new()
                .post(format!("{}/memory/add", base))
                .json(&serde_json::json!({
                    "actor": "alias-author",
                    "action": "noted",
                    "target": format!("alias {}", alias),
                    "record_type": alias
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status().as_u16(), 200, "alias {alias} must be accepted");

            // Accepted is not enough — the alias must have resolved to the
            // documented type rather than being coerced to something else.
            let ms = store.lock().unwrap();
            assert!(
                ms.find_by_actor("alias-author")
                    .iter()
                    .any(|r| r.record_type == expected),
                "alias {alias} did not resolve to {expected:?}"
            );
        }
    }

    // AC: the bulk path reports the bad index instead of dropping it silently.
    #[tokio::test]
    async fn bulk_add_reports_unknown_record_type() {
        let (state, _store) = make_test_state();
        let (base, _srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/bulk", base))
            .json(&serde_json::json!({
                "records": [
                    {"actor":"bulk","action":"wrote","target":"good","record_type":"Temporal"},
                    {"actor":"bulk","action":"wrote","target":"bad","record_type":"Nonsense"}
                ]
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let v: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(v["inserted"], 1, "the valid record must still land");
        assert_eq!(v["failed"], 1, "the invalid record must be counted as failed");
        let reason = v["errors"][0]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("Nonsense"),
            "bulk error must name the bad value, got {reason}"
        );
        assert_eq!(v["errors"][0]["index"], 1, "bulk error must name the index");
    }

    // AC: the read path accepts exactly the vocabulary the write path accepts.
    // A record must be readable back through the same `record_type` string that
    // created it — anything else makes the type system a write-only fiction.
    #[tokio::test]
    async fn query_record_type_shares_the_write_path_vocabulary() {
        let (state, _store) = make_test_state();
        let (base, _srv) = start_test_server(state).await;
        let client = reqwest::Client::new();

        for (actor, action, target, record_type) in [
            ("query-author", "planned", "ship it", "Goal"),
            ("query-author", "believed", "sky is blue", "semantic"),
        ] {
            let resp = client
                .post(format!("{}/memory/add", base))
                .json(&serde_json::json!({
                    "actor": actor,
                    "action": action,
                    "target": target,
                    "record_type": record_type
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(
                resp.status().as_u16(),
                200,
                "write path must accept {record_type}"
            );
        }

        // (alias as supplied on write, the type echoed back on read)
        for (alias, echoed) in [("Goal", "Goal"), ("semantic", "Symbolic")] {
            let resp = client
                .get(format!(
                    "{}/memory/query?actor=query-author&record_type={}",
                    base, alias
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(
                resp.status().as_u16(),
                200,
                "read path must accept {alias}, the alias the write path accepted"
            );
            let v: serde_json::Value = resp.json().await.unwrap();
            assert_eq!(
                v["total"], 1,
                "expected exactly one {echoed} record, got {}",
                v["total"]
            );
            assert_eq!(
                v["records"][0]["record_type"], echoed,
                "filtering by {alias} must return the {echoed} record it created"
            );
        }

        // An unknown type is still rejected — widening the vocabulary must not
        // turn the read path back into a silent-accept surface.
        let resp = client
            .get(format!(
                "{}/memory/query?actor=query-author&record_type=Fact",
                base
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 400, "an unknown type must be a 400");

        // Both records really exist and are not being hidden by a filter.
        let resp = client
            .get(format!("{}/memory/query?actor=query-author", base))
            .send()
            .await
            .unwrap();
        let v: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(v["total"], 2, "both records must be visible unfiltered");
    }

    // ── H9: GDPR erasure must be id-attributable ──────────────────────────────

    // AC: every destroyed id is disclosed, another actor is untouched, and the
    // erasure is journaled under an actor that survives it.
    #[tokio::test]
    async fn forget_actor_discloses_deleted_ids_and_journals() {
        let (state, store) = make_test_state();
        {
            let mut ms = store.lock().unwrap();
            for i in 0..3 {
                ms.add(MemoryRecord::new(
                    MemoryType::Temporal,
                    "gdpr-target".to_string(),
                    "wrote".to_string(),
                    format!("secret {i}"),
                    serde_json::json!({}),
                ))
                .unwrap();
            }
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "bystander".to_string(),
                "wrote".to_string(),
                "unrelated".to_string(),
                serde_json::json!({}),
            ))
            .unwrap();
        }
        let (base, _srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .delete(format!("{}/memory/forget/gdpr-target", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let v: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(v["records_deleted"], 3);

        let ids: Vec<String> = v["deleted_ids"]
            .as_array()
            .expect("deleted_ids must be present")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 3, "every erased record must be disclosed");

        let ms = store.lock().unwrap();
        assert!(
            ms.find_by_actor("gdpr-target").is_empty(),
            "the erased actor must have no records left"
        );
        assert_eq!(
            ms.find_by_actor("bystander").len(),
            1,
            "erasure must not reach other actors"
        );

        // The journal is written under "gdpr-erasure" so it outlives the erasure
        // it describes — this is what makes the deletion auditable.
        let journal = ms.find_by_actor("gdpr-erasure");
        assert_eq!(journal.len(), 1, "exactly one erasure journal record expected");
        assert_eq!(journal[0].action, "forget_actor");
        assert_eq!(journal[0].metadata["erased_actor"], "gdpr-target");
        assert_eq!(
            journal[0].metadata["deleted_ids"].as_array().unwrap().len(),
            3
        );
        for id in &ids {
            assert!(
                journal[0].metadata["deleted_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|x| x == id),
                "journal missing {id}"
            );
        }
    }

    // AC: erasing an actor with no records succeeds and tells the truth about it.
    #[tokio::test]
    async fn forget_absent_actor_is_idempotent() {
        let (state, _store) = make_test_state();
        let (base, _srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .delete(format!("{}/memory/forget/never-existed", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let v: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(v["records_deleted"], 0);
        assert_eq!(v["deleted_ids"].as_array().unwrap().len(), 0);
    }

    // ── Safety guardrail: classify content, never identifiers ────────────────

    // AC: a link whose UUIDs happen to read like a phone number is not refused as PII.
    // The guard exists to inspect content; a UUID is an opaque record handle that the
    // caller neither chooses nor can re-roll, so refusing one is a coin toss in prod.
    #[tokio::test]
    async fn link_does_not_classify_identifiers_as_pii() {
        let (state, _store) = make_test_state();
        let (base, srv) = start_test_server(state).await;

        // `123456-1234` is six digits, a hyphen, four digits — the
        // `\d{3}[-.\s]?\d{3}[-.\s]?\d{4}` shape the classifier scores as PII at 0.90 and
        // therefore blocks. Roughly one v4 UUID in twenty contains such a straddle.
        let tripping = "ab123456-1234-4abc-8def-0123456789ab";
        let other = "cd654321-4321-4fed-8cba-2109876543fe";

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/link", base))
            .json(&serde_json::json!({
                "from_id": tripping,
                "to_id": other,
                "relation": "supports"
            }))
            .send()
            .await
            .unwrap();

        // 403 is the classifier refusing the identifiers; 404 is the contract asserting
        // itself — the guard passed, the store was consulted, and these synthetic records
        // were not found. The status alone tells the two apart.
        assert_eq!(
            resp.status().as_u16(),
            404,
            "link refused before reaching the store: {}",
            resp.text().await.unwrap_or_default()
        );

        srv.abort();
    }
}
