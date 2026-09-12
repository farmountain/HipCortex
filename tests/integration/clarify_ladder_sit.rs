/// SIT: the clarify ladder over real HTTP (WP10, spec §4 AC + §5 test plan).
///
/// `tests/unit/clarify_ladder_tests.rs` calls the engine directly, so it proves the ladder's
/// *bounds* but says nothing about whether a client can reach the ladder or see it. The spec's
/// E2E clause is two claims about the **served router**, not about the engine:
///
/// > `/goal/:id/clarify` reachable; the ladder visible in `/goal/:id/trace`.
///
/// `acceptance_suite_v290::ac_g1` asserts those claims by grepping `src/web_server.rs` for a
/// string. A source scan cannot be wrong about behaviour — it stays green when the route is
/// unreachable in the served binary, when a guard never fires, or when the handler is a no-op
/// that merely *mentions* the right words. It also cannot see the particular defect this file
/// was written to catch: the clarify route used to merge caller-supplied acceptance criteria and
/// nothing else, so `POST /goal/:id/react`'s 422 redirect ("POST /goal/{id}/clarify") pointed at
/// an endpoint that changed nothing, answered 200, and returned the client to the same 422.
/// Reachable, yes — but not an exit.
///
/// So this file drives `build_app` over a socket and asserts: the ladder runs, it is visible in
/// the trace, the order is descending, the ledger is not mutated by reading it, and a goal that
/// still has no acceptance criteria cannot be react'd into a 200.
///
/// Uses a real TCP server + reqwest rather than `axum-test`, matching `route_parity_sit.rs`:
/// the `[dev-dependencies]` pin of `axum-test` is a different axum major than the `0.6` this
/// crate serves, so the in-process-server pattern is the honest harness here.
///
/// # Falsification record
///
/// Every assertion group below was proved load-bearing by disabling the code it guards and
/// confirming it reddens. A test that cannot fail is a comment with a `#[test]` on it.
///
/// | Probe | Disabled | Reddened |
/// |---|---|---|
/// | A | `if !supplied {` → `if false && !supplied {` in `/goal/:id/clarify` | `clarify_route_runs_the_ladder_and_the_trace_shows_it` (L191) + `react_on_a_goal_that_is_still_unclarifiable_is_rejected` (L344) |
/// | B | `"clarify_ladder": ladder` → `Value::Null` in `/goal/:id/trace` | the trace test (L228), `left: []` vs `right: ["T0_environment", "T1_prior_art", "T2_causal", "T3_ask_user"]` |
/// | C | `UNPROCESSABLE_ENTITY` → `OK` in the `still_uncheckable` arm of `/goal/:id/react` | the react test (L311), `left: 200` vs `right: 422` |
/// | D | `terminal && (succeeded \|\| !success_factors.is_empty())` → `terminal` | the react test (L344) — the redirect target stopped being observable |
/// | E | the `Failed` → `Pending` reset disabled | the react test (L370), `left: Some("Failed")` vs `right: Some("Pending")` |
/// | H | `wm_guard.as_deref()` → `None` in `/goal/:id/clarify` | `the_route_hands_the_live_world_model_to_the_causal_rung`, `left: "NeedsUserClarification …"` vs the expected `ClarifiedBySubstrate { source: Causal … }` — the ladder still descended and still asked, so nothing else in the file noticed |
/// | I | `build_report`'s `clarify_ladder` → `Vec::new()` | `the_report_shows_the_same_ladder_as_the_trace`, `left: []` vs `right: ["T0_environment", "T1_prior_art", "T2_causal", "T3_ask_user"]` |
///
/// Probe C is the one `ac_g1` provably cannot replace: a source scan of `src/web_server.rs` sees
/// the *string* `"goal must be clarified before react"` and stays green whatever status code is
/// returned around it. Only a client that reads the response can tell the two apart.
///
/// Probe H is the sibling for probes B/E and was the reason two tests were added rather than one:
/// the ladder had a *resolving* rung reachable only from a fixture, and no test asserted that the
/// route hands the rung the state it needs. A rung that can only ever answer `unavailable` is a
/// rung that does not exist, and the ladder would still look complete while it happened.
#[cfg(feature = "web-server")]
mod tests {
    use hipcortex::{
        archive_store::ArchiveStore,
        aureus_bridge::AureusBridge,
        clarify_engine::{
            ClarifyEngine, ClarifyOutcome, ClarifyTier, ClarifyTrigger, COST_OF_ASKING,
        },
        cognitive_gc::CognitiveGC,
        cognitive_state::CognitiveHandle,
        coherence::CoherenceChecker,
        memory_record::{MemoryRecord, MemoryType},
        memory_store::MemoryStore,
        payloads::{GoalPayload, GoalStatus, DEFAULT_GOAL_COST},
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
    use uuid::Uuid;

    type Store = Arc<Mutex<MemoryStore<InMemoryBackend>>>;
    type TestState = AppState<InMemoryBackend>;

    fn make_test_state() -> TestState {
        let memory_store: Store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
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

        AppState {
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
        }
    }

    /// Serve `build_app` on an ephemeral port and return its base URL.
    ///
    /// `build_app` specifically, because `bin/webserver.rs` serves that router — so anything
    /// asserted here is a claim about the production surface (spec §7 assumption 1), not about
    /// a router assembled for the test's convenience.
    async fn spawn_app_with(state: TestState) -> String {
        let app = build_app(state);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service())
                .await
                .unwrap_or_default();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        format!("http://127.0.0.1:{}", addr.port())
    }

    async fn spawn_app() -> String {
        spawn_app_with(make_test_state()).await
    }

    /// A server *plus* the two handles behind it, so a fixture can be seeded where the router
    /// will actually read it.
    ///
    /// `AppState` is moved into `build_app`, so the clones have to be taken first — but cloning
    /// the `Arc` hands back the router's own ownership, not a copy of the data, which is what
    /// makes the seeded state the state under test rather than a look-alike.
    async fn spawn_app_with_handles() -> (String, Store, Arc<RwLock<WorldModelEnhanced>>) {
        let state = make_test_state();
        let store = Arc::clone(&state.memory_store);
        let world_model = Arc::clone(&state.world_model);
        (spawn_app_with(state).await, store, world_model)
    }

    /// Link a `Temporal` step to a goal the way the ReAct loop does
    /// (`obs.derived_from = Some(goal_id)`, `src/modules/loop_engine.rs`).
    ///
    /// No REST route can express this: the ladder attributes over the goal's *own trajectory*, and
    /// `AddMemoryRequest` has no `derived_from` field. So a test that wants a goal carrying
    /// evidence has to seed it here.
    fn add_goal_linked_temporal(store: &Store, actor: &str, goal_id: Uuid, text: &str) {
        let mut step = MemoryRecord::new(
            MemoryType::Temporal,
            actor.to_string(),
            "observe".to_string(),
            text.to_string(),
            serde_json::Value::Null,
        );
        step.derived_from = Some(goal_id);
        store
            .lock()
            .unwrap()
            .add(step)
            .expect("seeding a trajectory step must succeed");
    }

    /// Create a goal through the public REST surface and return its id.
    ///
    /// Created the way a client creates it (`/memory/add` with `record_type: "goal"`), not by
    /// reaching into the store, so this also exercises the record-type mapping the ladder needs.
    async fn add_goal_via_rest(
        client: &reqwest::Client,
        base: &str,
        actor: &str,
        target: &str,
        estimated_cost: f64,
    ) -> String {
        let payload = GoalPayload {
            target_state: target.to_string(),
            estimated_cost,
            status: GoalStatus::Pending,
            ..Default::default()
        };
        let resp = client
            .post(format!("{base}/memory/add"))
            .json(&serde_json::json!({
                "actor": actor,
                "action": "set_goal",
                "target": target,
                "record_type": "goal",
                "metadata": serde_json::to_value(&payload).unwrap(),
            }))
            .send()
            .await
            .expect("POST /memory/add must be reachable");
        assert_eq!(
            resp.status().as_u16(),
            200,
            "goal creation via /memory/add failed: {}",
            resp.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        body.get("record_id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("AddMemoryResponse must carry record_id: {body}"))
            .to_string()
    }

    fn tiers_of(ladder: &serde_json::Value) -> Vec<String> {
        ladder
            .as_array()
            .map(|rs| {
                rs.iter()
                    .filter_map(|r| r.get("tier").and_then(|t| t.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── AC (§5): the clarify route drives the ladder, and the trace shows it ──────────────

    #[tokio::test]
    async fn clarify_route_runs_the_ladder_and_the_trace_shows_it() {
        let base = spawn_app().await;
        let client = reqwest::Client::new();
        // Cost 1.0 (not the `Default` 0.0) so the ask-cost gate is decided by declared risk
        // rather than by the absent-cost fallback: this test is about reachability and exit,
        // not about the gate, and the gate has its own teeth in the unit tests.
        let gid = add_goal_via_rest(&client, &base, "sit-clarify", "deploy_service", 1.0).await;

        let resp = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/clarify must be reachable");
        assert_eq!(
            resp.status().as_u16(),
            200,
            "the clarify route must answer; a 404 here is the H1a dead-router drift returning"
        );
        let body: serde_json::Value = resp.json().await.unwrap();

        // The route must report what the ladder concluded. Without this the endpoint answers
        // 200 to a request that changed nothing, which is how the 422 redirect loop hid.
        let outcome = body
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("the clarify route must report the ladder outcome: {body}"));
        assert!(
            outcome.contains("NeedsUserClarification"),
            "a goal with no AC could not be restated from anything, so the substrate must ask: {body}"
        );

        let tiers = tiers_of(body.get("clarify_ladder").unwrap_or(&serde_json::Value::Null));
        assert!(
            !tiers.is_empty(),
            "the route ran the ladder, so it must report the rungs: {body}"
        );
        assert_eq!(
            tiers.first().map(String::as_str),
            Some(ClarifyTier::T0Environment.as_str()),
            "the ladder is strictly descending, so T0 is always first: {tiers:?}"
        );
        assert!(
            tiers.contains(&ClarifyTier::T3AskUser.as_str().to_string()),
            "an unclarifiable goal must reach T3 — the only rung that asks a human: {tiers:?}"
        );

        // And the same ladder must be visible from the trace, because that is the clause the
        // spec actually names, and because a route that resolved the goal while hiding the
        // reasoning would leave an operator with a change they cannot audit.
        let trace: serde_json::Value = client
            .get(format!("{base}/goal/{gid}/trace"))
            .send()
            .await
            .expect("GET /goal/:id/trace must be reachable")
            .json()
            .await
            .unwrap();
        let traced = tiers_of(
            trace
                .get("clarify_ladder")
                .unwrap_or_else(|| panic!("trace must expose clarify_ladder: {trace}")),
        );
        assert_eq!(
            traced, tiers,
            "the trace must be a projection of the same ledger the clarify route read, not a \
             second opinion about it"
        );
        assert!(
            trace
                .get("records")
                .and_then(|v| v.as_array())
                .map(|r| !r.is_empty())
                .unwrap_or(false),
            "the projection is additive — the raw records a client already walks must survive: {trace}"
        );
        assert!(
            trace
                .get("clarify_exit_reasons")
                .and_then(|v| v.as_array())
                .is_some(),
            "the trace must also expose *why* the ladder stopped, so a human need not know the \
             internal `action` string of the Reflexion that carries it: {trace}"
        );
    }

    /// A caller that supplies acceptance criteria has answered the T3 question. That answer must
    /// be merged (backward compatible) and must *not* additionally spend ladder budget.
    #[tokio::test]
    async fn a_supplied_answer_is_merged_and_does_not_spend_ladder_budget() {
        let base = spawn_app().await;
        let client = reqwest::Client::new();
        let gid = add_goal_via_rest(&client, &base, "sit-answer", "deploy_service", 1.0).await;

        let resp: serde_json::Value = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({
                "success_factors": [
                    {"name": "service_up", "weight": 1.0, "satisfied": false,
                     "observation_pattern": "listening on 8080"}
                ]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            resp.get("success_factors").and_then(|v| v.as_u64()),
            Some(1),
            "the supplied criteria must be persisted: {resp}"
        );
        assert_eq!(
            resp.get("outcome").and_then(|v| v.as_str()),
            None,
            "an explicit human answer is the resolution — the ladder must not also run and \
             report a second, competing outcome: {resp}"
        );
        assert!(
            tiers_of(resp.get("clarify_ladder").unwrap_or(&serde_json::Value::Null)).is_empty(),
            "a goal clarified by its caller has no rungs to show: {resp}"
        );
    }

    // ── AC: a goal that cannot progress must not be react'd into a 200 ───────────────────

    #[tokio::test]
    async fn react_on_a_goal_that_is_still_unclarifiable_is_rejected() {
        let base = spawn_app().await;
        let client = reqwest::Client::new();
        // Cost 0.1 makes the ask-cost gate *decline* — the harder case for this guard, and the
        // one that proves the rejection keys off the goal's AC rather than off whether the
        // ladder happened to write a `clarify_needed` Belief. A decline resolves nothing and
        // writes no Belief, yet the goal still has no success factor, so a 200 here would tell
        // the client its goal is running when it provably cannot.
        let gid = add_goal_via_rest(&client, &base, "sit-react", "tweak_log_level", 0.1).await;

        let resp = client
            .post(format!("{base}/goal/{gid}/react"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/react must be reachable");

        assert_eq!(
            resp.status().as_u16(),
            422,
            "a goal with no success factor must be reported unprocessable and pointed at \
             /clarify, not answered 200"
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let err = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            err.contains("clarify"),
            "the rejection must name the route that fixes it: {body}"
        );

        // ── The named route must be a way out, which is the whole claim of this file ─────
        //
        // react → 422 → clarify. Two independent things have to hold for that to be an exit
        // rather than a loop, and the first of them was broken when this test was written.
        //
        // (i) The route must *run the ladder*. If it only merged caller-supplied AC, a client
        //     with no AC to supply — precisely the client that got the 422 — would merge
        //     nothing, receive a 200, and be sent back to the same 422 forever.
        let clarify: serde_json::Value = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(
            clarify.get("outcome").and_then(|v| v.as_str()).is_some(),
            "the redirect target must do something observable, or the 422 loop has no exit: {clarify}"
        );

        // (ii) The route must accept a goal the loop already gave up on. A goal failed *for
        //      want of an AC* is the canonical thing to clarify, so refusing it on the grounds
        //      that it is "completed" made the 422 advice unactionable and the failure
        //      permanent. Supplying the missing AC must therefore succeed — and must leave
        //      the goal runnable again, or the repair is a formality that cannot be used.
        let repaired: serde_json::Value = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({"success_factors": [
                {"name": "log_level_is_tweakable", "weight": 1.0, "satisfied": false}
            ]}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            repaired.get("success_factors").and_then(|v| v.as_u64()),
            Some(1),
            "a failed goal with no AC must be repairable by supplying one: {repaired}"
        );
        assert_eq!(
            repaired.get("status").and_then(|v| v.as_str()),
            Some("Pending"),
            "a repaired goal must stop reporting itself as finished, or the next /react \
             fails and the 422 → clarify → react cycle never terminates: {repaired}"
        );
    }

    // ── G8: the 409 must name the exit that exists, and that exit must work ──────────────

    /// A goal the ladder already settled, which the loop then failed, is not "completed".
    ///
    /// `/clarify` is right to refuse it: `success_factors` exist, so there is nothing left to
    /// clarify. But the caller is not stuck either — `/react` only answers 422 when
    /// `success_factors` is empty, so retrying against the settled factors is the supported
    /// path. The 409 therefore has to name it; a bare "cannot clarify completed goal" misstates
    /// the state *and* hides the exit.
    #[tokio::test]
    async fn a_settled_failed_goal_answers_409_naming_the_retry_that_works() {
        let base = spawn_app().await;
        let client = reqwest::Client::new();

        // Built as the wire payload and round-tripped through `GoalPayload`, per the payload
        // rule: metadata is only ever read back through the typed struct.
        let metadata = serde_json::json!({
            "target_state": "tweak_log_level",
            "estimated_cost": 1.0,
            "status": "Failed",
            "success_factors": [
                {"name": "log_level_is_tweakable", "weight": 1.0, "satisfied": false}
            ],
        });
        assert!(
            serde_json::from_value::<GoalPayload>(metadata.clone()).is_ok(),
            "the fixture must round-trip through GoalPayload: {metadata}"
        );

        let created: serde_json::Value = client
            .post(format!("{base}/memory/add"))
            .json(&serde_json::json!({
                "actor": "sit-settled",
                "action": "set_goal",
                "target": "tweak_log_level",
                "record_type": "goal",
                "metadata": metadata,
            }))
            .send()
            .await
            .expect("POST /memory/add must be reachable")
            .json()
            .await
            .unwrap();
        let gid = created
            .get("record_id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("AddMemoryResponse must carry record_id: {created}"))
            .to_string();

        let clarify = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/clarify must be reachable");
        assert_eq!(
            clarify.status().as_u16(),
            409,
            "a settled goal has nothing left to clarify and must be refused"
        );
        let body: serde_json::Value = clarify.json().await.unwrap();
        let err = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            err.contains("success_factors"),
            "the refusal must name what is already settled: {body}"
        );
        assert!(
            !err.contains("completed"),
            "a Failed goal is not a completed one — that wording made a recoverable state \
             read as terminal: {body}"
        );
        let fix = body
            .get("fix")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            fix.contains(&format!("/goal/{gid}/react")),
            "the refusal must name the supported retry *and this goal's id*: {body}"
        );

        // The named exit must be reachable. `/react` rejects only a goal with no factor to
        // check; this one has one, so following the advice cannot land on the same dead end.
        let react = client
            .post(format!("{base}/goal/{gid}/react"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/react must be reachable");
        assert_eq!(
            react.status().as_u16(),
            200,
            "the route named by the 409 must accept the goal it pointed at: {}",
            react.text().await.unwrap_or_default()
        );
    }

    // ── The ledger is a pure read, and an unclarified goal still answers ─────────────────

    #[test]
    fn reading_the_ladder_never_mutates_it() {
        let state = make_test_state();
        let payload = GoalPayload {
            target_state: "deploy_service".into(),
            estimated_cost: 1.0,
            status: GoalStatus::Pending,
            ..Default::default()
        };
        let mut rec = hipcortex::memory_record::MemoryRecord::new(
            hipcortex::memory_record::MemoryType::Goal,
            "sit-ledger".to_string(),
            "set_goal".to_string(),
            "deploy_service".to_string(),
            serde_json::to_value(&payload).unwrap(),
        );
        rec.actor = "sit-ledger".into();
        let goal_id = rec.id;
        state.memory_store.lock().unwrap().add(rec).unwrap();

        let mut s = state.memory_store.lock().unwrap();
        let outcome = ClarifyEngine::run(
            &mut s,
            goal_id,
            "sit-ledger",
            ClarifyTrigger::EmptyAC,
            None,
        );
        assert_eq!(
            outcome,
            ClarifyOutcome::NeedsUserClarification,
            "no environment signal, no prior art, no world model — the ladder must end by asking"
        );

        let first = ClarifyEngine::ladder_rungs(&s, goal_id);
        assert_eq!(
            first.len(),
            1 + ClarifyTier::search_rungs().len(),
            "one rung per search tier plus the terminal T3 rung"
        );
        for _ in 0..3 {
            let again = ClarifyEngine::ladder_rungs(&s, goal_id);
            assert_eq!(
                first.len(),
                again.len(),
                "a projection that writes is not a projection — repeated reads must not add rungs"
            );
        }
        // The stop reason on a *natural* exhaustion is carried by the T3 rung itself, which
        // records `asked`. `clarify_exit_reasons` is for the *bounds* — budget, lifetime,
        // no-progress — where the ladder stopped before it ran out of rungs, and the unit tests
        // pin each bound's attribution separately. So a first ask must record its rung and
        // invent no bound, keeping the two ledgers answerable to different questions.
        let last = first
            .last()
            .expect("the ladder always records its terminal rung");
        assert_eq!(
            last.tier, "T3_ask_user",
            "the last rung must be the ask, not a search tier"
        );
        assert_eq!(
            last.outcome, "asked",
            "the rung must record that the substrate asked, rather than leave a client to \
             infer it from the outcome alone"
        );
        assert!(
            ClarifyEngine::ladder_exit_reasons(&s, goal_id).is_empty(),
            "a natural exhaustion hits no bound, so it must not fabricate one"
        );
    }

    /// A net for the two constants the tests above depend on. If either were replaced by a
    /// vacuous value, the HTTP tests could pass for the wrong reason — `clarify_route_runs...`
    /// would still see a ladder while the ask-cost gate had silently stopped asking.
    #[test]
    fn the_constants_these_assertions_depend_on_are_still_load_bearing() {
        assert!(
            DEFAULT_GOAL_COST > COST_OF_ASKING,
            "the gate must admit a goal at the declared default cost, or the T3 rung the HTTP \
             test asserts could never fire"
        );
        assert_eq!(
            ClarifyTier::search_rungs().len(),
            3,
            "the ledger length assertions assume exactly three search tiers"
        );
        assert!(
            !ClarifyTier::search_rungs().contains(&ClarifyTier::T3AskUser),
            "T3 is terminal; if it were a search rung the descent loop would reach its \
             `unreachable!` arm and panic"
        );
    }

    // ── §5 coverage: the rung that *resolves*, and the report's copy of the ladder ─────────

    /// T2 — the causal rung — over the served router.
    ///
    /// `tests/unit/clarify_ladder_tests::ladder_t2_resolves_from_broken_structural_equation`
    /// hands the engine `Some(&wm)`, so it proves the rung *works*. No test proved that the route
    /// ever *gives* it a world model. That is not a hypothetical: the handler reaches the world
    /// model through `clarify_wm` inside a closure, so `wm_guard.as_deref()` → `None` keeps every
    /// other assertion in this file green while silently demoting T2 to `unavailable` for every
    /// client — the ladder would still descend, still ask, and still look correct. A rung that
    /// can only ever answer "no world model" is a rung that does not exist.
    #[tokio::test]
    async fn the_route_hands_the_live_world_model_to_the_causal_rung() {
        let (base, store, world_model) = spawn_app_with_handles().await;
        let client = reqwest::Client::new();
        let actor = "sit-causal";
        let gid = add_goal_via_rest(&client, &base, actor, "drain_the_queue", 1.0).await;
        let goal_id = Uuid::parse_str(&gid).expect("the route returns a uuid");

        // Two preconditions of the rung cannot be built through the public REST surface, and that
        // is a property of the surface rather than a convenience of the fixture:
        //
        //   * `POST /worldmodel/causal/edge` creates nodes *without* a structural equation, and
        //     `credit_assign` only ever scores a node that has one — so a causal graph built by
        //     curl is, by construction, unattributable;
        //   * `AddMemoryRequest` carries no `derived_from`, so only the ReAct loop links a
        //     `Temporal` step to a goal — and the loop only runs for a goal that already has a
        //     decidable AC, which is exactly the goal the ladder is not run for.
        //
        // So the equation and the trajectory are seeded through the same `Arc`s the router holds.
        // The claim under test is still about the served route: that it consults the live world
        // model rather than `None`.
        //
        // Arithmetic (`causal::LinearSE`): `invert_for_u(&[], observed) == observed`, so
        // `score = mean |observed|` and `confidence = score / (score + 1)`. Six mentions per step
        // scores 6.0 and clears the 0.85 gate, which needs `score >= 5.667`.
        const NODE: &str = "queue_depth";
        {
            let wm = world_model.read().unwrap();
            wm.add_causal_node(NODE.to_string())
                .expect("adding a causal node must succeed");
            wm.rewrite_structural_equation(NODE, vec![])
                .expect("rewriting the equation is the only way to give a node one");
        }
        for _ in 0..3 {
            add_goal_linked_temporal(
                &store,
                actor,
                goal_id,
                "queue_depth queue_depth queue_depth queue_depth queue_depth queue_depth",
            );
        }

        let resp: serde_json::Value = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/clarify must be reachable")
            .json()
            .await
            .unwrap();

        let outcome = resp
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            outcome.contains("ClarifiedBySubstrate") && outcome.contains("Causal"),
            "the goal's own trajectory mentions one equation-bearing node above the confidence \
             gate, so the route's live world model must resolve this at T2 rather than ask the \
             user: {resp}"
        );

        let ladder = resp
            .get("clarify_ladder")
            .unwrap_or_else(|| panic!("the clarify response must carry the ladder: {resp}"));
        let t2 = ladder
            .as_array()
            .and_then(|rs| {
                rs.iter().find(|r| {
                    r.get("tier").and_then(|t| t.as_str()) == Some("T2_causal")
                })
            })
            .unwrap_or_else(|| panic!("T2 must have run and been recorded: {ladder}"));
        assert_eq!(
            t2.get("outcome").and_then(|o| o.as_str()),
            Some("resolved"),
            "a rung that ran and resolved must say so: {ladder}"
        );
        let evidence: Vec<String> = t2
            .get("evidence")
            .and_then(|e| e.as_array())
            .map(|es| {
                es.iter()
                    .filter_map(|e| e.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            evidence
                .iter()
                .any(|e| e.contains("broken_equation=queue_depth")),
            "the resolution has to name the equation it blames, or a human cannot check the \
             claim the substrate just made about its own failure: {evidence:?}"
        );
        assert!(
            evidence.iter().any(|e| e.contains("confidence=")),
            "the confidence the gate was cleared by must be visible, or the gate cannot be \
             audited: {evidence:?}"
        );
        assert!(
            !tiers_of(ladder).iter().any(|t| t == "T3_ask_user"),
            "the substrate explained the failure — asking the user afterwards is exactly the \
             waste the ladder exists to avoid: {ladder}"
        );
    }

    /// The report must not carry a second, divergent copy of the ladder.
    ///
    /// Q10 answers "what should happen next" and the ladder answers "why did the substrate not
    /// decide by itself". A human reading only the report sees `recommended_op: "clarify_goal"`
    /// and nothing else, so the report has to project the *same* ledger the trace does — if it
    /// computed its own ladder, the two answers could disagree and neither would be wrong.
    #[tokio::test]
    async fn the_report_shows_the_same_ladder_as_the_trace() {
        let base = spawn_app().await;
        let client = reqwest::Client::new();
        let actor = "sit-report-ladder";
        let gid = add_goal_via_rest(&client, &base, actor, "deploy_service", 1.0).await;

        let clarify: serde_json::Value = client
            .post(format!("{base}/goal/{gid}/clarify"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("POST /goal/:id/clarify must be reachable")
            .json()
            .await
            .unwrap();
        let from_route = tiers_of(clarify.get("clarify_ladder").unwrap_or_else(|| {
            panic!("the clarify response must carry the ladder: {clarify}")
        }));
        assert!(
            !from_route.is_empty(),
            "the ladder must have run, or the parity below is the parity of two empty lists: \
             {clarify}"
        );

        let trace: serde_json::Value = client
            .get(format!("{base}/goal/{gid}/trace"))
            .send()
            .await
            .expect("GET /goal/:id/trace must be reachable")
            .json()
            .await
            .unwrap();
        let from_trace = tiers_of(
            trace
                .get("clarify_ladder")
                .unwrap_or(&serde_json::Value::Null),
        );

        let report: serde_json::Value = client
            .get(format!("{base}/v1/cognitive/report?actor={actor}"))
            .send()
            .await
            .expect("GET /v1/cognitive/report must be reachable")
            .json()
            .await
            .unwrap();
        let from_report = tiers_of(report.get("clarify_ladder").unwrap_or_else(|| {
            panic!("the report must carry the ladder, including when empty: {report}")
        }));

        assert_eq!(
            from_report, from_route,
            "the report and the route must be two projections of one ledger, not two opinions \
             about it: report={from_report:?} route={from_route:?} trace={from_trace:?}"
        );
        assert_eq!(
            from_trace, from_route,
            "…and so must the trace: trace={from_trace:?} route={from_route:?}"
        );
        assert_eq!(
            report
                .get("next_recommendation")
                .and_then(|n| n.get("recommended_op"))
                .and_then(|v| v.as_str()),
            Some("clarify_goal"),
            "a goal with no decidable AC is not runnable, so the report must say so rather than \
             recommend react_loop: {report}"
        );
    }
}
