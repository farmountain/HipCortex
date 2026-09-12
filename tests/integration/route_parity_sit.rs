/// SIT: route-table parity guard (H1c).
///
/// `run_with_both_stores` proved the failure mode: a second router nobody
/// served owned 9 routes, so `POST /goal/:id/clarify` returned **404** in
/// production while `GET /goal/:id/trace` returned 200, and the OpenAPI document
/// had drifted to 39 documented paths against 123 live ones. Nothing in the
/// build caught either, because axum 0.6's `Router` has no route-table accessor
/// — `route`, `nest`, `merge` and `layer` all return `Self` and nothing
/// enumerates.
///
/// Since the router cannot be introspected at runtime, the guard works in three
/// independent directions instead:
///
/// 1. **Complete** — parse the `.route(...)` calls out of `web_server.rs` and
///    require an exact match with `openapi_spec::ROUTE_TABLE`, plus a route-call
///    count check so a route the parser cannot classify (e.g. a future `any()`)
///    fails loudly instead of being dropped.
/// 2. **Sound** — probe every declared path in a live in-process server and
///    require it to dispatch. The probe uses `TRACE`, which no route registers,
///    so it never reaches a handler body and cannot mutate test state. A
///    negative control on a nonsense path proves the probe can actually tell
///    "registered" from "not registered".
/// 3. **Documented** — require the served OpenAPI document to describe every
///    declared route, and to describe nothing that is not declared.
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
        memory_store::MemoryStore,
        openapi_spec::{canonical_path, spec_with_route_table, ROUTE_TABLE},
        persistence::InMemoryBackend,
        self_model::{calibration::CalibrationTracker, SelfModel},
        substrate_daemon::SubstrateDaemon,
        symbolic_store::SymbolicStore,
        topological_memory::CausalTopoGraph,
        web_server::{build_app, AppState},
        workspace::WorkspaceRegistry,
        world_model_enhanced::WorldModelEnhanced,
    };
    use std::collections::{BTreeSet, HashMap};
    use std::sync::{Arc, Mutex, RwLock};

    const SOURCE: &str = include_str!("../../src/web_server.rs");
    const VERBS: [&str; 5] = ["get", "post", "put", "delete", "patch"];

    type Store = Arc<Mutex<MemoryStore<InMemoryBackend>>>;
    type TestState = AppState<InMemoryBackend>;

    // ── source parsing ───────────────────────────────────────────────────────

    /// Replace comment bytes with spaces, preserving string literals and line
    /// structure. Comments are blanked rather than removed so that a commented
    /// out `.route(...)` is not counted as a live route and so that an
    /// unbalanced paren inside prose cannot desynchronise paren depth tracking.
    fn strip_comments(src: &str) -> String {
        let b = src.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(b.len());
        let mut i = 0usize;
        while i < b.len() {
            let c = b[i];
            if c == b'"' {
                out.push(c);
                i += 1;
                while i < b.len() {
                    let d = b[i];
                    out.push(d);
                    i += 1;
                    if d == b'\\' {
                        if i < b.len() {
                            out.push(b[i]);
                            i += 1;
                        }
                        continue;
                    }
                    if d == b'"' {
                        break;
                    }
                }
            } else if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    out.push(b' ');
                    i += 1;
                }
            } else if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                while i < b.len() {
                    if b[i] == b'*' && i + 1 < b.len() && b[i + 1] == b'/' {
                        out.push(b' ');
                        out.push(b' ');
                        i += 2;
                        break;
                    }
                    out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
            } else {
                out.push(c);
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    /// If `(` at `paren` is the call paren of a routing verb, return that verb.
    ///
    /// Handles both the leading form (`get(handler)`) and the chained form
    /// (`get(a).post(b)`) — the chained form is why a naive "first verb wins"
    /// parser reports `/worldmodel/predict` as GET-only when it is really
    /// GET+POST. A `.verb(` is only accepted when the token before the dot is
    /// `)`, i.e. a MethodRouter chain, so `map.get(&k)` is not mistaken for a
    /// route method.
    fn verb_call_at(chunk: &str, paren: usize) -> Option<&'static str> {
        let b = chunk.as_bytes();
        let mut j = paren;
        while j > 0 && b[j - 1].is_ascii_whitespace() {
            j -= 1;
        }
        let ident_end = j;
        while j > 0 && (b[j - 1].is_ascii_alphanumeric() || b[j - 1] == b'_') {
            j -= 1;
        }
        let ident = &chunk[j..ident_end];
        let verb = VERBS.iter().copied().find(|v| *v == ident)?;
        if j > 0 && b[j - 1] == b'.' {
            let mut k = j - 1;
            while k > 0 && b[k - 1].is_ascii_whitespace() {
                k -= 1;
            }
            if !(k > 0 && b[k - 1] == b')') {
                return None;
            }
        }
        Some(verb)
    }

    fn verbs_in(chunk: &str) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for (i, byte) in chunk.bytes().enumerate() {
            if byte == b'(' {
                if let Some(v) = verb_call_at(chunk, i) {
                    if !out.contains(&v) {
                        out.push(v);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// End of the top-level statement starting at `from`, respecting nesting and
    /// string literals.
    fn statement_end(b: &[u8], from: usize) -> usize {
        let mut depth = 0i32;
        let mut in_str = false;
        let mut i = from;
        while i < b.len() {
            let c = b[i];
            if in_str {
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'{' || c == b'(' || c == b'[' {
                depth += 1;
            } else if c == b'}' || c == b')' || c == b']' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if c == b';' && depth == 0 {
                break;
            }
            i += 1;
        }
        i
    }

    /// Verbs of the `let <name> = ...` binding for a route variable, e.g.
    /// `let wm_predict_route = { ... get(..).post(..) };`.
    fn let_bound_verbs(region: &str, name: &str) -> Vec<&'static str> {
        let b = region.as_bytes();
        let mut search = 0usize;
        while let Some(rel) = region[search..].find("let ") {
            let mut p = search + rel + 4;
            search = p;
            while p < b.len() && b[p].is_ascii_whitespace() {
                p += 1;
            }
            if region[p..].starts_with("mut ") {
                p += 4;
            }
            let id_start = p;
            while p < b.len() && (b[p].is_ascii_alphanumeric() || b[p] == b'_') {
                p += 1;
            }
            if &region[id_start..p] != name {
                continue;
            }
            let mut q = p;
            while q < b.len() && b[q] != b'=' && b[q] != b';' {
                q += 1;
            }
            if q >= b.len() || b[q] != b'=' {
                continue;
            }
            let end = statement_end(b, q + 1);
            return verbs_in(&region[q + 1..end]);
        }
        Vec::new()
    }

    fn is_ident(s: &str) -> bool {
        !s.is_empty()
            && s.as_bytes()[0].is_ascii_alphabetic()
            && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    }

    /// `(route_call_count, {(METHOD, path)})` for every `.route(...)` in
    /// `build_app` — the router that is actually served.
    fn parse_build_app_routes() -> (usize, BTreeSet<(String, String)>) {
        let src = strip_comments(SOURCE);
        let start = src
            .find("pub fn build_app")
            .expect("web_server.rs must define build_app");
        let end = src
            .find("pub async fn run_with_store")
            .expect("web_server.rs must define run_with_store");
        assert!(start < end, "build_app must be declared before run_with_store");
        let region = &src[start..end];
        let b = region.as_bytes();

        let mut rows: BTreeSet<(String, String)> = BTreeSet::new();
        let mut route_calls = 0usize;
        let mut unresolved: Vec<String> = Vec::new();
        let mut pos = 0usize;
        while let Some(rel) = region[pos..].find(".route(") {
            let at = pos + rel;
            pos = at + ".route(".len();
            route_calls += 1;

            let mut p = pos;
            while p < b.len() && b[p].is_ascii_whitespace() {
                p += 1;
            }
            assert_eq!(
                b[p], b'"',
                "every .route( must start with a string-literal path (byte {p} in build_app)"
            );
            let q = region[p + 1..].find('"').expect("route path literal") + p + 1;
            let path = region[p + 1..q].to_string();

            // handler chunk: up to the paren that closes `.route(`
            let mut depth = 1usize;
            let mut i = q + 1;
            let mut in_str = false;
            while i < b.len() && depth > 0 {
                let c = b[i];
                if in_str {
                    if c == b'\\' {
                        i += 2;
                        continue;
                    }
                    if c == b'"' {
                        in_str = false;
                    }
                } else if c == b'"' {
                    in_str = true;
                } else if c == b'(' {
                    depth += 1;
                } else if c == b')' {
                    depth -= 1;
                }
                i += 1;
            }
            let handler = &region[q + 1..i - 1];

            let mut verbs = verbs_in(handler);
            if verbs.is_empty() {
                let ident = handler.trim().trim_start_matches(',').trim();
                if is_ident(ident) {
                    verbs = let_bound_verbs(region, ident);
                }
            }
            if verbs.is_empty() {
                unresolved.push(format!("{} (line ~{})", path, route_calls));
                continue;
            }
            for v in verbs {
                rows.insert((v.to_uppercase(), path.clone()));
            }
        }

        assert!(
            unresolved.is_empty(),
            "{} route(s) in build_app could not be attributed to a verb — add them to \
             ROUTE_TABLE by hand and extend the parser: {unresolved:#?}",
            unresolved.len()
        );
        (route_calls, rows)
    }

    fn table_rows() -> BTreeSet<(String, String)> {
        ROUTE_TABLE
            .iter()
            .map(|(m, p)| ((*m).to_string(), (*p).to_string()))
            .collect()
    }

    // ── 1. completeness ──────────────────────────────────────────────────────

    /// AC: `ROUTE_TABLE` is exactly the route surface of `build_app`.
    #[test]
    fn route_table_matches_build_app() {
        let (route_calls, declared) = parse_build_app_routes();
        let table = table_rows();

        let declared_paths: BTreeSet<&String> = declared.iter().map(|(_, p)| p).collect();
        let table_paths: BTreeSet<&String> = table.iter().map(|(_, p)| p).collect();

        assert_eq!(
            route_calls,
            table_paths.len(),
            "build_app registers {route_calls} .route(...) calls but ROUTE_TABLE declares {} \
             distinct paths. A route the parser could not classify is invisible to every client \
             that generates from /openapi.json.",
            table_paths.len()
        );
        assert_eq!(
            declared_paths, table_paths,
            "path set drifted between build_app and ROUTE_TABLE"
        );
        assert_eq!(
            declared, table,
            "method set drifted between build_app and ROUTE_TABLE"
        );
    }

    /// AC: the declared table is unambiguous — no path is declared twice with the
    /// same method, which would silently shadow one handler with another.
    #[test]
    fn route_table_is_free_of_duplicate_entries() {
        let mut seen: HashMap<(&str, &str), usize> = HashMap::new();
        for (i, row) in ROUTE_TABLE.iter().enumerate() {
            if let Some(prev) = seen.insert(*row, i) {
                panic!("ROUTE_TABLE[{i}] {:?} duplicates ROUTE_TABLE[{prev}]", row);
            }
        }
        assert!(
            ROUTE_TABLE.len() >= 120,
            "route surface shrank to {} entries — expected the full build_app surface",
            ROUTE_TABLE.len()
        );
    }

    // ── 3. documented ────────────────────────────────────────────────────────

    /// AC: the served document describes every route the server actually serves.
    #[test]
    fn served_spec_describes_every_declared_route() {
        let doc = spec_with_route_table();
        let paths = doc["paths"]
            .as_object()
            .expect("served spec has a paths object");

        let mut documented: BTreeSet<(String, String)> = BTreeSet::new();
        for (key, item) in paths {
            let Some(obj) = item.as_object() else { continue };
            for verb in obj.keys() {
                if VERBS.contains(&verb.as_str()) {
                    documented.insert((verb.to_uppercase(), canonical_path(key)));
                }
            }
        }

        let missing: Vec<&(&str, &str)> = ROUTE_TABLE
            .iter()
            .filter(|(m, p)| !documented.contains(&(m.to_string(), canonical_path(p))))
            .collect();
        assert!(
            missing.is_empty(),
            "{} declared route(s) are absent from the served OpenAPI document: {missing:#?}",
            missing.len()
        );
    }

    /// AC: the document advertises nothing that is not a live route.
    #[test]
    fn spec_documents_no_path_absent_from_the_router() {
        let declared: BTreeSet<String> = ROUTE_TABLE
            .iter()
            .map(|(_, p)| canonical_path(p))
            .collect();
        let doc = spec_with_route_table();
        let paths = doc["paths"].as_object().expect("paths object");

        let stale: Vec<&String> = paths
            .keys()
            .filter(|k| !declared.contains(&canonical_path(k)))
            .collect();
        assert!(
            stale.is_empty(),
            "the served spec documents {} path(s) that build_app never registers, so clients \
             will generate calls that 404: {stale:#?}",
            stale.len()
        );
    }

    // ── 2. soundness (runtime) ───────────────────────────────────────────────

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

    /// Substitute every `:param` with a fixed placeholder so the probe URL is
    /// routable. Handlers are never reached, so the value never matters.
    fn probe_url(base: &str, path: &str) -> String {
        let mut out = String::from(base);
        for seg in path.split('/') {
            if seg.is_empty() {
                continue;
            }
            out.push('/');
            if let Some(rest) = seg.strip_prefix(':') {
                if !rest.is_empty() {
                    out.push_str("probe");
                    continue;
                }
            }
            out.push_str(seg);
        }
        out
    }

    /// AC: every declared route dispatches. `TRACE` is registered by no route,
    /// so a match yields 405 and a miss yields 404 without any handler running.
    #[tokio::test]
    async fn every_declared_route_dispatches() {
        let app = build_app(make_test_state());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let _srv = tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service())
                .await
                .unwrap_or_default();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        let base = format!("http://127.0.0.1:{}", addr.port());
        let client = reqwest::Client::new();
        let probe = reqwest::Method::from_bytes(b"TRACE").unwrap();

        // Negative control: without this, a blanket 401/500 from a middleware
        // would make every route look registered and the test prove nothing.
        let control = client
            .request(probe.clone(), format!("{base}/__no_such_route__"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            control.status().as_u16(),
            404,
            "probe has no discriminating power: an unregistered path answered {} instead of 404",
            control.status()
        );

        let mut missing: Vec<&str> = Vec::new();
        for (_, path) in ROUTE_TABLE.iter() {
            let url = probe_url(&base, path);
            let resp = client
                .request(probe.clone(), &url)
                .send()
                .await
                .unwrap_or_else(|e| panic!("probe {url} failed to send: {e}"));
            if resp.status().as_u16() == 404 {
                missing.push(path);
            }
        }
        assert!(
            missing.is_empty(),
            "{} route(s) declared in ROUTE_TABLE 404 in build_app — the table advertises \
             endpoints the server does not serve, or a route was registered outside this \
             router: {missing:#?}",
            missing.len()
        );
    }

    /// AC: the restored dead-router routes answer. Guards the specific H1a
    /// regression — 9 routes existed only on `run_with_both_stores`, so they
    /// 404'd in production.
    #[tokio::test]
    async fn formerly_dead_routes_are_reachable() {
        for path in [
            "/goal/:id/clarify",
            "/goal/:id/verify",
            "/memory/diff",
            "/v1/actions/authorized-wm",
            "/v1/loop/omega",
            "/v1/loop/status/:handle",
            "/v1/loop/stop/:handle",
            "/v1/loop/subscribe",
            "/v1/workspace/:id/renew",
        ] {
            assert!(
                ROUTE_TABLE.iter().any(|(_, p)| *p == path),
                "{path} is missing from ROUTE_TABLE — the dead-router drift is back"
            );
        }
    }
}
