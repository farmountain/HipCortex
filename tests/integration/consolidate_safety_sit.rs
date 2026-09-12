/// SIT: `POST /memory/consolidate` safety contract.
///
/// Consolidation deletes records. Before this suite the endpoint was
/// `dry_run`-optional, accepted an anonymous caller, compared records with a
/// containment ratio (`|A∩B| / max(|A|,|B|)`) instead of Jaccard, and could mark
/// a record for dropping and then elect it as a survivor in the same pass —
/// three independent paths to silent data loss.
///
/// Every assertion below is a goals-driven acceptance criterion from the
/// remediation spec (§4 WP1). Uses a real TCP server + reqwest to avoid
/// axum-test / axum-0.6 version skew.
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
    type Archive = Arc<Mutex<ArchiveStore>>;
    type TestState = AppState<InMemoryBackend>;

    fn make_test_state() -> (TestState, Store, Archive) {
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
        let archive_store: Archive = Arc::new(Mutex::new(ArchiveStore::new_in_memory()));
        let archive_ref = archive_store.clone();

        let state = AppState {
            memory_store,
            symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
            world_model,
            aureus: Arc::new(Mutex::new(AureusBridge::new())),
            self_model,
            coherence,
            topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
            archive_store,
            tx_log: None,
            calibration,
            cognitive,
            forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: false,
        };
        (state, store_ref, archive_ref)
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

    /// Three byte-identical records => Jaccard 1.0 => one cluster of size 3.
    fn seed_triple(store: &Store, actor: &str) {
        let mut ms = store.lock().unwrap();
        for _ in 0..3 {
            let mut r = MemoryRecord::new(
                MemoryType::Temporal,
                actor.to_string(),
                "decided".to_string(),
                "use postgres for auth".to_string(),
                serde_json::json!({"note": "postgres chosen for the auth store"}),
            );
            // Deterministic confidences so survivor election is assertable.
            r.confidence = 0.5;
            ms.add(r).unwrap();
        }
    }

    fn records_for(store: &Store, actor: &str) -> Vec<MemoryRecord> {
        store
            .lock()
            .unwrap()
            .find_by_actor(actor)
            .into_iter()
            .cloned()
            .collect()
    }

    async fn call(base: &str, query: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
        let resp = reqwest::Client::new()
            .post(format!("{}/memory/consolidate?{}", base, query))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = resp.status().as_u16();
        let value = resp.json::<serde_json::Value>().await.unwrap_or_default();
        (status, value)
    }

    // AC-1: an anonymous caller cannot consolidate anything.
    #[tokio::test]
    async fn actor_is_required() {
        let (state, store, _archive) = make_test_state();
        seed_triple(&store, "alice");
        let (base, _srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/consolidate", base))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        assert!(
            resp.status().is_client_error(),
            "omitting actor must be rejected, got {}",
            resp.status()
        );
        assert_eq!(records_for(&store, "alice").len(), 3, "store must be untouched");
    }

    // AC-2: preview is the default; nothing is destroyed without an explicit ack.
    #[tokio::test]
    async fn dry_run_is_the_default() {
        let (state, store, archive) = make_test_state();
        seed_triple(&store, "alice");
        let (base, _srv) = start_test_server(state).await;

        let (status, v) = call(&base, "actor=alice", serde_json::json!({})).await;

        assert_eq!(status, 200);
        assert_eq!(v["dry_run"], true, "dry_run must default to true");
        assert_eq!(v["mutated"], false, "default call must not mutate");
        assert_eq!(v["candidate_records"], 3);
        assert_eq!(v["clusters_found"], 1);
        assert_eq!(v["duplicates_found"], 2);
        assert_eq!(v["archived"], 0);
        assert_eq!(records_for(&store, "alice").len(), 3, "preview must not delete");
        assert_eq!(archive.lock().unwrap().record_count(), 0, "preview must not archive");
    }

    // AC-3: `dry_run=false` alone is not consent.
    #[tokio::test]
    async fn mutation_requires_both_dry_run_false_and_confirm() {
        let (state, store, _archive) = make_test_state();
        seed_triple(&store, "alice");
        let (base, _srv) = start_test_server(state).await;

        let (status, v) = call(&base, "actor=alice&dry_run=false", serde_json::json!({})).await;

        assert_eq!(status, 200);
        assert_eq!(v["mutated"], false, "confirm is mandatory");
        assert_eq!(records_for(&store, "alice").len(), 3);
    }

    // AC-4: the happy path archives before deleting, and the survivor inherits provenance.
    #[tokio::test]
    async fn mutation_archives_then_deletes_and_survivor_absorbs_evidence() {
        let (state, store, archive) = make_test_state();
        seed_triple(&store, "alice");
        let (base, _srv) = start_test_server(state).await;

        let (status, v) = call(
            &base,
            "actor=alice&dry_run=false&confirm=true",
            serde_json::json!({}),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(v["mutated"], true);
        assert_eq!(v["clusters_found"], 1);
        assert_eq!(v["duplicates_found"], 2);
        assert_eq!(v["archived"], 2, "both duplicates must reach the Cold Store");
        assert!(v["errors"].as_array().unwrap().is_empty(), "errors: {}", v["errors"]);

        let archived_ids: Vec<String> = v["archived_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(archived_ids.len(), 2);

        // Cold Store holds exactly the two dropped ids — recoverability proof.
        let cold_ids: Vec<String> = archive
            .lock()
            .unwrap()
            .load_all()
            .unwrap()
            .iter()
            .map(|r| r.id.to_string())
            .collect();
        assert_eq!(cold_ids.len(), 2);
        for id in &archived_ids {
            assert!(cold_ids.contains(id), "archived id {id} missing from the Cold Store");
        }

        // Hot Store: 1 survivor + 1 journal Reflexion, and never fewer than one record.
        let hot = records_for(&store, "alice");
        assert_eq!(hot.len(), 2, "expected survivor + journal, got {:?}", hot.len());
        let survivor = hot
            .iter()
            .find(|r| r.action == "decided")
            .expect("survivor must remain in the Hot Store");
        assert_eq!(survivor.evidence.len(), 2, "survivor must absorb both dropped ids");
        for id in &archived_ids {
            assert!(
                survivor.evidence.iter().any(|u| u.to_string() == *id),
                "survivor.evidence missing {id}"
            );
        }
        assert_eq!(
            survivor.metadata["consolidated_from"].as_array().unwrap().len(),
            2,
            "survivor must record what it absorbed"
        );

        // The journal names every kept and archived id so the op is attributable.
        let journal = hot
            .iter()
            .find(|r| r.action == "consolidate")
            .expect("a consolidate journal record must be written");
        assert_eq!(journal.metadata["archived"].as_array().unwrap().len(), 2);
        assert_eq!(journal.metadata["kept"].as_array().unwrap().len(), 1);
    }

    // AC-5: consolidation is never cross-actor.
    #[tokio::test]
    async fn never_crosses_actor_boundary() {
        let (state, store, _archive) = make_test_state();
        seed_triple(&store, "alice");
        seed_triple(&store, "bob");
        let (base, _srv) = start_test_server(state).await;

        let (_, v) = call(
            &base,
            "actor=alice&dry_run=false&confirm=true",
            serde_json::json!({}),
        )
        .await;

        assert_eq!(v["candidate_records"], 3, "only alice's records are candidates");
        assert_eq!(records_for(&store, "bob").len(), 3, "bob must be untouched");
    }

    // AC-6: a pinned record is never merged into a normal one.
    #[tokio::test]
    async fn never_merges_pinned_with_unpinned() {
        let (state, store, _archive) = make_test_state();
        {
            let mut ms = store.lock().unwrap();
            let mut pinned = MemoryRecord::new(
                MemoryType::Temporal,
                "alice".to_string(),
                "decided".to_string(),
                "use postgres for auth".to_string(),
                serde_json::json!({"note": "postgres chosen for the auth store"}),
            );
            pinned.priority = "pinned".to_string();
            ms.add(pinned).unwrap();
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "alice".to_string(),
                "decided".to_string(),
                "use postgres for auth".to_string(),
                serde_json::json!({"note": "postgres chosen for the auth store"}),
            ))
            .unwrap();
        }
        let (base, _srv) = start_test_server(state).await;

        let (_, v) = call(&base, "actor=alice", serde_json::json!({})).await;

        assert_eq!(v["candidate_records"], 2);
        assert_eq!(
            v["clusters_found"], 0,
            "the pinned/normal boundary must block clustering"
        );
    }

    // AC-7: a Temporal record is never merged into a Symbolic one.
    #[tokio::test]
    async fn never_crosses_record_type() {
        let (state, store, _archive) = make_test_state();
        {
            let mut ms = store.lock().unwrap();
            for rt in [MemoryType::Temporal, MemoryType::Symbolic] {
                ms.add(MemoryRecord::new(
                    rt,
                    "alice".to_string(),
                    "decided".to_string(),
                    "use postgres for auth".to_string(),
                    serde_json::json!({"note": "postgres chosen for the auth store"}),
                ))
                .unwrap();
            }
        }
        let (base, _srv) = start_test_server(state).await;

        let (_, v) = call(&base, "actor=alice", serde_json::json!({})).await;

        assert_eq!(v["candidate_records"], 2);
        assert_eq!(v["clusters_found"], 0, "types must not be merged");
    }

    // AC-8: unrelated records are never clustered (Jaccard, not containment).
    #[tokio::test]
    async fn unrelated_records_do_not_cluster() {
        let (state, store, _archive) = make_test_state();
        {
            let mut ms = store.lock().unwrap();
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "alice".to_string(),
                "decided".to_string(),
                "use postgres for auth".to_string(),
                serde_json::json!({}),
            ))
            .unwrap();
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "alice".to_string(),
                "decided".to_string(),
                "rewrite the vector index in rust and benchmark recall".to_string(),
                serde_json::json!({"owner": "search-team"}),
            ))
            .unwrap();
        }
        let (base, _srv) = start_test_server(state).await;

        let (_, v) = call(&base, "actor=alice", serde_json::json!({})).await;

        assert_eq!(
            v["clusters_found"], 0,
            "a short target must not clear the threshold by containment"
        );
    }

    /// G4: the handler must mutate through the bulk primitives, not the per-record one.
    ///
    /// axum 0.6's `Router` exposes no handler or route table, so the handler's own source is the
    /// only accessible artifact. The trailing parenthesis is load-bearing: a naive
    /// `!body.contains("delete_by_id")` would also match `delete_by_ids(`.
    #[test]
    fn consolidation_uses_bulk_primitives_not_delete_by_id() {
        let source = include_str!("../../src/web_server.rs");
        let lines: Vec<&str> = source.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.contains("async fn handle_consolidate"))
            .expect("handle_consolidate must exist");
        let end = lines[start + 1..]
            .iter()
            .position(|l| {
                !l.starts_with(' ')
                    && (l.starts_with("async fn ")
                        || l.starts_with("pub async fn ")
                        || l.starts_with("fn ")
                        || l.starts_with("pub fn ")
                        || l.starts_with("impl ")
                        || l.starts_with("struct ")
                        || l.starts_with('}'))
            })
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());
        let body = lines[start..end].join("\n");

        assert!(
            body.len() > 200,
            "the slice must contain a handler body, got {} chars",
            body.len()
        );
        assert!(
            body.contains("upsert("),
            "the survivor must be replaced through `upsert`, not `delete_by_id` + `add`"
        );
        assert!(
            body.contains("delete_by_ids("),
            "the duplicates must be dropped with one bulk removal"
        );
        assert!(
            !body.contains("delete_by_id("),
            "no per-record `delete_by_id(` may remain in the handler: it is neither durable nor bulk"
        );
    }

    /// G5: the safety invariant over a seeded sweep of cluster shapes.
    ///
    /// The property-test harness cannot host this one: `tests/property/` aliases
    /// `MemoryStore<InMemoryBackend>` and adding `web-server` there would feature-unify the
    /// property target away from the minimal build CI enforces. It is enumerated
    /// deterministically here instead -- `groups` clusters of `per_group` byte-identical
    /// records, 64 shapes in total.
    #[tokio::test]
    async fn invariant_holds_over_seeded_cluster_shapes() {
        let mut shapes = 0;

        for groups in 1usize..=8 {
            for per_group in 1usize..=8 {
                shapes += 1;
                let ctx = || format!("groups={} per_group={}", groups, per_group);

                let (state, store, archive) = make_test_state();
                {
                    let mut ms = store.lock().unwrap();
                    for g in 0..groups {
                        for k in 0..per_group {
                            let mut r = MemoryRecord::new(
                                MemoryType::Temporal,
                                "alice".to_string(),
                                "decided".to_string(),
                                format!("topic{}", g),
                                serde_json::json!({}),
                            );
                            // Unique per record so survivor election is deterministic. Confidence
                            // is not part of the similarity token set, so a group still scores
                            // Jaccard 1.0 internally.
                            r.confidence = 0.5_f32 + (k as f32) * 0.01;
                            ms.add(r).unwrap();
                        }
                    }
                }

                let before: Vec<uuid::Uuid> = records_for(&store, "alice")
                    .iter()
                    .map(|r| r.id)
                    .collect();
                assert_eq!(before.len(), groups * per_group, "seed count ({})", ctx());

                let (base, _srv) = start_test_server(state).await;
                let (status, v) = call(
                    &base,
                    "actor=alice&dry_run=false&confirm=true",
                    serde_json::json!({}),
                )
                .await;
                assert_eq!(status, 200, "status ({})", ctx());

                let after_records: Vec<MemoryRecord> = records_for(&store, "alice");
                // The handler journals the call as a single Reflexion record owned by the same
                // actor, so the *live* set excludes it.
                let journal_count = after_records
                    .iter()
                    .filter(|r| r.action == "consolidate")
                    .count();
                let active: Vec<&MemoryRecord> = after_records
                    .iter()
                    .filter(|r| r.action != "consolidate")
                    .collect();
                let active_ids: Vec<uuid::Uuid> = active.iter().map(|r| r.id).collect();
                let has_clusters = per_group >= 2;
                let read_ids = |field: &str| -> Vec<uuid::Uuid> {
                    v[field]
                        .as_array()
                        .unwrap_or_else(|| panic!("`{}` must be an array ({})", field, ctx()))
                        .iter()
                        .map(|s| uuid::Uuid::parse_str(s.as_str().unwrap()).unwrap())
                        .collect()
                };
                let survivor_ids = read_ids("survivors");
                let archived_ids = read_ids("archived_ids");

                // 0. Exactly one journal record, and only when a cluster was actually merged.
                assert_eq!(
                    journal_count,
                    usize::from(has_clusters),
                    "journal record count ({})",
                    ctx()
                );
                // 1. The live set never grows. Growth is the symptom the non-durable delete caused.
                assert!(
                    active_ids.len() <= before.len(),
                    "consolidation must never grow the Hot Store ({})",
                    ctx()
                );
                // 2. Exactly one record per group survives, whatever the group size.
                assert_eq!(
                    active_ids.len(),
                    groups,
                    "exactly one live record per group ({})",
                    ctx()
                );
                // A singleton group is not a cluster, so no survivor is *elected* for it: the
                // record simply stays put and is reported as neither survivor nor archive.
                let expected_survivors = if has_clusters { groups } else { 0 };
                assert_eq!(
                    survivor_ids.len(),
                    expected_survivors,
                    "elected survivors ({})",
                    ctx()
                );
                // 3. Every id reported as surviving is still live, and election picked the
                //    highest-confidence member of its group.
                for id in &survivor_ids {
                    let rec = active.iter().find(|r| r.id == *id).unwrap_or_else(|| {
                        panic!("reported survivor {} is not live ({})", id, ctx())
                    });
                    assert_eq!(
                        rec.confidence,
                        0.5_f32 + ((per_group - 1) as f32) * 0.01,
                        "survivor {} must be the highest-confidence member ({})",
                        id,
                        ctx()
                    );
                }
                // 4. No id can be both dropped and surviving.
                for id in &archived_ids {
                    assert!(
                        !active_ids.contains(id),
                        "archived record {} is still live ({})",
                        id,
                        ctx()
                    );
                }
                // 5. Every dropped record is recoverable from the Cold Store.
                let cold = archive.lock().unwrap().load_all().unwrap();
                let cold_ids: Vec<uuid::Uuid> = cold.iter().map(|r| r.id).collect();
                assert_eq!(cold_ids.len(), archived_ids.len(), "cold count ({})", ctx());
                for id in &archived_ids {
                    assert!(
                        cold_ids.contains(id),
                        "archived {} is not in the Cold Store ({})",
                        id,
                        ctx()
                    );
                }
                // 6. Conservation: nothing left the live set without being archived first.
                assert_eq!(
                    active_ids.len() + archived_ids.len(),
                    before.len(),
                    "every removed record must have been archived ({})",
                    ctx()
                );
                assert_eq!(
                    archived_ids.len(),
                    groups * per_group.saturating_sub(1),
                    "one archive per duplicate, none for singletons ({})",
                    ctx()
                );
                // 7. The reported counters agree with the observed mutation.
                assert_eq!(
                    v["clusters_found"].as_u64().unwrap() as usize,
                    expected_survivors,
                    "clusters_found ({})",
                    ctx()
                );
                assert_eq!(
                    v["duplicates_found"].as_u64().unwrap() as usize,
                    archived_ids.len(),
                    "duplicates_found ({})",
                    ctx()
                );
            }
        }

        assert_eq!(shapes, 64, "the sweep must cover all 64 shapes");
    }
}
