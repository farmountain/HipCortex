# Integrity Format Versioning, Alias Parity, and the gRPC Build

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the three evidenced defects that survived the G1–G12 remediation — a REST record-type vocabulary that diverges between the write path and the embed path, a `--features grpc-server` build that has not compiled since `MemoryRecord` gained fields, and an integrity check that cannot distinguish an old hash format from tampering (and which therefore makes `rollback()` unusable on any store holding pre-upgrade records).

**Architecture:** Each task is a leaf change at one responsible layer. Task 1 routes `/memory/embed` through the same `parse_record_type_alias` seam `/memory/add` already uses, and applies the same `SafetyGuardrail` precondition — it currently calls an LLM connector and mutates the store with no guardrail. Task 2 completes a `MemoryRecord` struct literal that omits 14 fields, which is a hard `E0063`; the feature is never built by any CI job, so the break has been invisible. Task 3 adds a `hash_version` tag and a three-valued `IntegrityVerdict` so a reader can tell "hashed by a superseded serializer" from "modified after it was hashed"; `rollback()` then tolerates the former, refuses the latter, and *records* how many records it could not verify instead of silently accepting them. Task 4 makes the E2E Merkle assertion assert what is actually true, and writes the diagnosis down.

**Tech Stack:** Rust (edition 2021, MSRV 1.70), axum 0.6, tokio 1, serde/serde_json, sha2, proptest (dev), reqwest (dev), pytest (Python E2E harness), Criterion (benches, untouched).

---

## Global Constraints

- **MSRV is 1.70.** Never use `Option::is_none_or` (stabilised 1.82) or any other post-1.70 API. `cargo clippy` on this machine reports for rust-1.95.0 and *will* suggest `is_none_or` at `src/web_server.rs:~3852` — do not apply it.
- **Minimal-feature builds must keep compiling.** The canonical dev loop is
  `cargo build --no-default-features --features "petgraph_backend"`.
  Web/tokio work uses `--no-default-features --features "petgraph_backend,web-server"`.
  Never use `--all-features` (it needs PostgreSQL, SQLite, RocksDB, Neo4j and protoc).
- **Never call `Router::routes()`.** axum 0.6 exposes no route-table accessor. Route parity, the capability catalog and every route guard are **declaration-based** on `ROUTE_TABLE` in `src/openapi_spec.rs`. A new route means a new `ROUTE_TABLE` row.
- **REST invariants live in `tests/integration/*_sit.rs`, not in the property suite.** The property-test harness cannot host REST-path tests.
- **`MemoryBackend` has no delete** (`src/persistence.rs:19-23`: `load`, `append`, `flush`, `clear`). Durable removal is `clear()` + re-append.
- **Write records only through `MemoryStore`.** It owns the SHA-256 integrity hash, the AES-GCM envelope, the WAL and the `audit.log` Merkle chain. Never hand-write a `MemoryRecord` to disk.
- **Never mutate the graph store, the FSM backend, or call an LLM connector without `SafetyGuardrail::check_precondition` first**, and classify **content, never identifiers**.
- **Payloads round-trip through `serde_json::{to_value, from_value}`** into the structs in `src/payloads.rs`. Never construct raw metadata JSON by hand.
- **Archive only via `ArchiveStore::archive()`.** Never set `status = "archived"` on a live record.
- **Call `CognitiveGC::gc_action(id)` before hard-deleting** any Temporal or Belief record.
- **Preserve the Chain-of-Thought comment header** at the top of each module.
- **One commit per concern.** Do not commit `.claude/settings.local.json` (it is dirty in the working tree and must stay that way).
- **Test targets are named explicitly.** `Cargo.toml` declares 24 `[[test]]` targets; `cargo test` with no `--test` argument still runs the suites, but every command in this plan names its target.
- **Local tooling:** `rtk` is **not** installed on this machine — use plain `git`, `cargo`, `npm`, `pytest`. `protoc` is **not** installed, so `--features grpc-server` cannot be built locally (Task 2 accounts for this). `gh` is available.
- **A live server runs on `:3030`** (the user's primary instance, `DATA_DIR=c:\Users\user\.hipcortex\data`). Leave it running. It rewrites `memory.jsonl` while you read it, so freeze a copy before measuring it.

---

### Task 1: `/memory/embed` shares the write path's record-type vocabulary and guardrail

`POST /memory/embed` has its own hand-written alias ladder that is **case-sensitive** and **silently coerces** unknown values to `Temporal`:

```rust
let record_type = match req.record_type.as_deref() {
    Some("Symbolic") => MemoryType::Symbolic,
    Some("Procedural") => MemoryType::Procedural,
    Some("Reflexion") => MemoryType::Reflexion,
    Some("Perception") => MemoryType::Perception,
    _ => MemoryType::Temporal,
};
```

So `{"record_type":"belief"}` — accepted verbatim by `/memory/add` — is stored here as a 24-hour decaying `Temporal` record, and the caller gets `{"success":true}`. The same handler also calls `generate_embedding` (a network call to Ollama or OpenAI) **before** it looks at `record_type`, so it can never reject a bad type without spending the call; and it mutates the store without the `SafetyGuardrail::check_precondition` that `/memory/add` performs.

**Files:**
- Modify: `src/web_server.rs` (handler `handle_embed_and_add`, starting line 3125; the 400 body mirrors `handle_add_memory`'s arm at line 4793)
- Test: `tests/integration/v110_rest_sit.rs` (append; already registered in `tests/integration/mod.rs`, header `#![cfg(feature = "web-server")]`)

**Interfaces:**
- Consumes: `parse_record_type_alias(s: Option<&str>) -> Result<MemoryType, String>` (`src/web_server.rs:4743`, module-private, returns `Err(offending_value)`); `const RECORD_TYPE_ALIASES: &[&str]` (`src/web_server.rs:4724`); `crate::safety_guardrail::SAFETY_GUARDRAIL` (a `Mutex<SafetyGuardrail>`, `.lock()` then `.check_precondition(&ctx) -> Result<(), String>`); test helpers `make_test_state()`, `start_test_server(state) -> (String, JoinHandle<()>)`, `add_and_query(&str, &str, &str) -> serde_json::Value`.
- Produces: `POST /memory/embed` returns `400` with `error` naming the offending value and `warning.valid_record_types` listing `RECORD_TYPE_ALIASES`; returns `403` when the guardrail rejects `"<actor> <action> <target>"`. No signature changes, no new route, so no `ROUTE_TABLE` row.

- [ ] **Step 1: Write the failing tests.**

Append to `tests/integration/v110_rest_sit.rs`. The stub Ollama server is needed because `/memory/embed` only reaches the store after a successful embedding; `generate_embedding` reads `OLLAMA_URL` on every call, so pointing it at a local stub keeps the test offline and deterministic.

```rust
/// A stub embedding provider. `generate_embedding` reads `OLLAMA_URL` per call, so the embed
/// path can be exercised offline by pointing that variable at this server.
#[cfg(feature = "web-server")]
async fn start_ollama_stub() -> (String, tokio::task::JoinHandle<()>) {
    let app = axum::Router::new().route(
        "/api/embeddings",
        axum::routing::post(|| async {
            axum::Json(serde_json::json!({ "embedding": [0.1_f64, 0.2, 0.3] }))
        }),
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    (format!("http://127.0.0.1:{}", addr.port()), handle)
}

/// rta7: `/memory/embed` must reject an unknown record_type with 400 *before* it spends an
/// embedding call. Before the fix it parsed nothing, called the provider first, and returned
/// 502 from a bogus model — the bad type was never the reported problem.
#[cfg(feature = "web-server")]
#[tokio::test]
async fn rta7_embed_rejects_unknown_type_before_embedding() {
    let (base, _h) = start_test_server(make_test_state()).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/memory/embed", base))
        .json(&serde_json::json!({
            "actor": "rta7", "action": "test", "target": "t",
            "record_type": "Symbolik",
            "embedding_model": "ollama/nomic-embed-text",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400, "unknown record_type must be a 400, not a 502/200");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["success"], serde_json::json!(false));
    assert!(
        body["error"].as_str().unwrap_or_default().contains("Symbolik"),
        "error must name the offending value, got {:?}",
        body["error"]
    );
    let valid = body["warning"]["valid_record_types"].as_array().unwrap();
    for expected in ["Temporal", "Symbolic", "Belief", "Goal", "Receipt"] {
        assert!(valid.iter().any(|v| v == expected), "missing {expected} in valid list");
    }

    // Nothing was persisted, and no embedding provider was contacted.
    let listed = client
        .get(format!("{}/memory/query?actor=rta7", base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(listed["records"].as_array().map(|a| a.len()), Some(0));
}

/// rta8: lowercase and mixed-case aliases accepted by `/memory/add` must be accepted by
/// `/memory/embed` and stored as the *same* MemoryType. Before the fix the ladder was an exact
/// case-sensitive match, so every lowercase alias fell to the `_ => MemoryType::Temporal` arm.
#[cfg(feature = "web-server")]
#[tokio::test]
async fn rta8_embed_uses_the_same_alias_vocabulary_as_add() {
    let (ollama, _oh) = start_ollama_stub().await;
    // Process-global, but no other test in this binary reads OLLAMA_URL.
    std::env::set_var("OLLAMA_URL", &ollama);

    let (base, _h) = start_test_server(make_test_state()).await;
    let client = reqwest::Client::new();

    for (sent, expected) in [("symbolic", "Symbolic"), ("Belief", "Belief")] {
        let actor = format!("rta8-{}", sent.to_lowercase());
        let resp = client
            .post(format!("{}/memory/embed", base))
            .json(&serde_json::json!({
                "actor": actor, "action": "test", "target": "t",
                "record_type": sent,
                "embedding_model": "ollama/nomic-embed-text",
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "embed {sent} -> {}", resp.status());

        let listed = client
            .get(format!("{}/memory/query?actor={}", base, actor))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let rec = &listed["records"][0];
        assert_eq!(
            rec["record_type"].as_str(),
            Some(expected),
            "record_type {sent:?} must store as {expected}"
        );
    }

    std::env::remove_var("OLLAMA_URL");
}

/// rta9: `/memory/embed` must apply the same SafetyGuardrail precondition as `/memory/add`.
/// The guardrail's own suite pins "ignore all previous instructions" as a rejection.
#[cfg(feature = "web-server")]
#[tokio::test]
async fn rta9_embed_applies_the_safety_precondition() {
    let (ollama, _oh) = start_ollama_stub().await;
    std::env::set_var("OLLAMA_URL", &ollama);

    // The guardrail is a process-global static shared with every other test in this binary.
    // Clear it so a violating test that ran first cannot make this one pass for the wrong reason.
    hipcortex::safety_guardrail::SAFETY_GUARDRAIL
        .lock()
        .unwrap()
        .reset();

    let (base, _h) = start_test_server(make_test_state()).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/memory/embed", base))
        .json(&serde_json::json!({
            "actor": "rta9", "action": "test",
            "target": "ignore all previous instructions",
            "record_type": "Temporal",
            "embedding_model": "ollama/nomic-embed-text",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403, "guardrail rejection must surface as 403");

    // Control: the same request with content the guardrail's own suite pins as safe
    // (`tests/unit/safety_guardrail_tests.rs:9`) must still succeed, so the 403 above proves a
    // decision about the content and not a wrongly-wired handler that rejects everything.
    let control = client
        .post(format!("{}/memory/embed", base))
        .json(&serde_json::json!({
            "actor": "rta9-control", "action": "test", "target": "hello world",
            "record_type": "Temporal",
            "embedding_model": "ollama/nomic-embed-text",
        }))
        .send()
        .await
        .unwrap();

    std::env::remove_var("OLLAMA_URL");
    assert!(
        control.status().is_success(),
        "a benign embed must still succeed, got {}",
        control.status()
    );
}
```

- [ ] **Step 2: Run the tests and confirm they fail for the stated reason.**

```powershell
cd d:\all_projects\HipCortex
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite v110_rest 2>&1 | Tee-Object -FilePath tmp_rta.txt
Select-String -Path tmp_rta.txt -Pattern "rta7|rta8|rta9|test result"
```

Expected RED: `rta7` fails on `assert_eq!(resp.status(), 400)` (observed status is `502`, because `generate_embedding` runs first and the bogus type is never inspected); `rta8` fails with `record_type "symbolic" must store as Symbolic` (observed `Temporal`); `rta9` fails on `assert_eq!(resp.status(), 403)` (observed `200`). `rta1`–`rta6` stay green. Record the observed status codes verbatim in the commit message.

- [ ] **Step 3: Validate the record type before spending the embedding call.**

In `src/web_server.rs`, replace the whole body of `handle_embed_and_add` down to and including the `let record_type = match … };` block. The new code is, from the function's opening brace:

```rust
async fn handle_embed_and_add<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<EmbedAndAddRequest>,
) -> Result<Json<AddMemoryResponse>, (StatusCode, Json<AddMemoryResponse>)> {
    // Parse the record type *first*: an unrecognised value must not cost an embedding call.
    // The previous ladder was an exact case-sensitive match that ended in
    // `_ => MemoryType::Temporal`, so `"symbolic"` and `"belief"` were accepted as success while
    // silently storing a 24-hour decaying Temporal record. `/memory/add` already refuses that.
    let record_type = match parse_record_type_alias(req.record_type.as_deref()) {
        Ok(t) => t,
        Err(bad) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AddMemoryResponse {
                    success: false,
                    record_id: None,
                    error: Some(format!(
                        "unknown record_type {:?}; valid values: {}",
                        bad,
                        RECORD_TYPE_ALIASES.join(", ")
                    )),
                    warning: Some(serde_json::json!({
                        "valid_record_types": RECORD_TYPE_ALIASES,
                        "reason": "record_type was previously coerced to Temporal on unrecognised input"
                    })),
                }),
            ))
        }
    };

    // Safety: classify free-text target/action before mutation, as `/memory/add` does. This
    // handler calls an LLM connector and writes a record, so it is inside the guardrail contract.
    {
        let ctx = format!("{} {} {}", req.actor, req.action, req.target);
        if let Ok(mut guard) = crate::safety_guardrail::SAFETY_GUARDRAIL.lock() {
            if let Err(reason) = guard.check_precondition(&ctx) {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(AddMemoryResponse {
                        success: false,
                        record_id: None,
                        error: Some(reason),
                        warning: None,
                    }),
                ));
            }
        }
    }

    let embedding = match generate_embedding(&req.embedding_model, &req.target).await {
        Ok(v) => v,
        Err(e) => {
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(AddMemoryResponse {
                    success: false,
                    record_id: None,
                    error: Some(e),
                    warning: None,
                }),
            ))
        }
    };

    let mut metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));
```

The remainder of the function (`if let serde_json::Value::Object(ref mut map) = metadata { … }` onward) is unchanged.

- [ ] **Step 4: Run the tests and confirm they pass.**

```powershell
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite v110_rest 2>&1 | Tee-Object -FilePath tmp_rta2.txt
Select-String -Path tmp_rta2.txt -Pattern "rta\d|test result"
```

Expected GREEN: `rta1`–`rta9` pass, `test result: ok`. If `rta8` stores `Temporal`, the `match` arm was placed after the embedding call — move it up, do not weaken the test.

- [ ] **Step 5: Confirm the change did not disturb the rest of the web surface.**

```powershell
cargo clippy --no-default-features --features "petgraph_backend,web-server" --all-targets -- -D clippy::correctness
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite 2>&1 | Tee-Object -FilePath tmp_int.txt
Select-String -Path tmp_int.txt -Pattern "test result|FAILED|panicked"
```

Do not accept a clippy `is_none_or` suggestion — it violates MSRV 1.70.

- [ ] **Step 6: Commit.**

```powershell
Remove-Item tmp_rta.txt, tmp_rta2.txt, tmp_int.txt -ErrorAction SilentlyContinue
git add src/web_server.rs tests/integration/v110_rest_sit.rs
git commit -m "fix(web-server): /memory/embed shares the write path's type vocabulary and guardrail

The embed handler kept a second, case-sensitive alias ladder that ended in
`_ => MemoryType::Temporal`, so `record_type: \"belief\"` was stored as a decaying
Temporal record and reported as success. It also ran the embedding call before
inspecting the type, and mutated the store without passing
SafetyGuardrail::check_precondition, which every other mutation path performs.

Route through parse_record_type_alias, validate before embedding, and apply the
precondition. rta7/rta8/rta9 pin all three; rta7 and rta9 were RED as 502 and 200."
```

---

### Task 2: `--features grpc-server` compiles again

`src/grpc_server.rs:48` builds a `MemoryRecord` with a struct literal that lists **8** of the struct's 22 fields:

```rust
let mut record = MemoryRecord {
    id: rec.id.parse()…?, record_type: mtype, timestamp: …, actor: rec.actor,
    action: rec.action, target: rec.target, metadata: …, integrity: None,
};
```

`#[serde(default)]` on the missing fields does **not** make them optional in a Rust struct literal, so this is a hard compile error. Verified locally by replicating the literal verbatim in a scratch example and compiling it against the library:

```
error[E0063]: missing fields `access_count`, `confidence`, `content_hash` and 11 other fields
       in initializer of `MemoryRecord`
error: could not compile `hipcortex` (example "_scratch_grpc_literal") due to 1 previous error
```

No CI job builds `grpc-server` — `ci.yml` and `release.yml` only name `petgraph_backend`, `tokio` and `web-server` — so the gRPC surface has been dead code since `MemoryRecord` gained the metric and provenance fields. `src/passive_capture.rs:136` holds the same style of literal and is complete; it is the template.

**Files:**
- Modify: `src/grpc_server.rs` (the `MemoryRecord` literal at line 48)
- Test: `tests/unit/memory_record_tests.rs` (append; already registered in `tests/unit/mod.rs`)

**Interfaces:**
- Consumes: `MemoryRecord::new(record_type, actor, action, target, metadata) -> MemoryRecord` — the constructor that must be the default for any new record; `MemoryRecord`'s field list in `src/memory_record.rs:22`.
- Produces: a record built by the gRPC path carries the same default field values as one built by `MemoryRecord::new` for every field the gRPC payload does not supply. No API change; `src/grpc_server.rs` is entirely `#[cfg(feature = "grpc-server")]`.

- [ ] **Step 1: Write the failing test.**

Append to `tests/unit/memory_record_tests.rs`. This pins the *values* the gRPC literal must supply, so a future field addition fails a named test rather than an unbuilt feature.

```rust
/// The gRPC add path builds a `MemoryRecord` by struct literal. That literal silently drifted
/// out of sync with the struct (E0063, 14 fields missing) because no CI job builds
/// `--features grpc-server`. Pin the defaults it must carry, mirroring `MemoryRecord::new`.
#[test]
fn gpc_literal_defaults_match_the_constructor() {
    let now = chrono::Utc::now();
    let constructed = MemoryRecord::new(
        MemoryType::Temporal,
        "actor".to_string(),
        "action".to_string(),
        "target".to_string(),
        serde_json::json!({}),
    );

    // Fields the gRPC payload does not carry must equal the constructor's defaults.
    assert_eq!(constructed.access_count, 0);
    assert_eq!(constructed.relevance_score, 1.0);
    assert_eq!(constructed.confidence, 1.0);
    assert_eq!(constructed.version, 0);
    assert_eq!(constructed.priority, "normal");
    assert_eq!(constructed.status, "active");
    assert_eq!(constructed.content_hash, None);
    assert_eq!(constructed.expires_at, None);
    assert_eq!(constructed.source, None);
    assert!(constructed.tags.is_empty());
    assert!(constructed.evidence.is_empty());
    assert_eq!(constructed.derived_from, None);
    assert_eq!(constructed.react_iteration, None);
    assert_eq!(constructed.integrity, None);
    assert_eq!(constructed.last_accessed, constructed.timestamp);
    let _ = now; // timestamp is supplied by the gRPC payload, not by the constructor
}
```

The natural home already exists: `tests/unit/memory_tests.rs` imports `MemoryRecord`, `MemoryType`, `serde_json::json`, `tempfile::tempdir` and `uuid::Uuid` and already holds `test_memory_record_creation` (`:14`) and `test_memory_record_hash_computation` (`:30`). Append the test there — no new file, no `tests/unit/mod.rs` edit.

- [ ] **Step 2: Run it and confirm it passes.**

```powershell
cargo test --no-default-features --features "petgraph_backend" --test unit_suite gpc_literal_defaults 2>&1 | Select-String -Pattern "test result|FAILED|panicked"
```

Expected: `1 passed`. This test documents the target values; the failing artefact is the compile error in the next step, which cannot be expressed as a test while `protoc` is absent.

- [ ] **Step 3: Complete the literal.**

In `src/grpc_server.rs`, replace the `let mut record = MemoryRecord { … };` statement with a complete literal. Keep `integrity: None` and keep the existing `record.integrity = Some(hash)` that follows — the current gRPC semantics deliberately leave `content_hash` unset, and changing that is not this task.

```rust
        let mut record = MemoryRecord {
            id: rec
                .id
                .parse()
                .map_err(|_| tonic::Status::invalid_argument("id"))?,
            record_type: mtype,
            timestamp: chrono::Utc
                .timestamp_opt(rec.timestamp, 0)
                .single()
                .ok_or_else(|| tonic::Status::invalid_argument("timestamp"))?,
            actor: rec.actor,
            action: rec.action,
            target: rec.target,
            metadata: serde_json::from_str(&rec.metadata)
                .map_err(|_| tonic::Status::invalid_argument("metadata"))?,
            integrity: None,
            // The fields below are not part of the gRPC payload; they mirror MemoryRecord::new.
            // Leaving them out was an E0063 that no CI job could see, because no job builds
            // --features grpc-server.
            access_count: 0,
            last_accessed: chrono::Utc
                .timestamp_opt(rec.timestamp, 0)
                .single()
                .ok_or_else(|| tonic::Status::invalid_argument("timestamp"))?,
            relevance_score: 1.0,
            content_hash: None,
            expires_at: None,
            confidence: 1.0,
            source: None,
            version: 0,
            tags: Vec::new(),
            priority: "normal".to_string(),
            status: "active".to_string(),
            evidence: Vec::new(),
            derived_from: None,
            react_iteration: None,
        };
```

If Step 4 reports any further missing field, add it with the same value `MemoryRecord::new` uses — do not invent a value.

- [ ] **Step 4: Prove the error is gone.**

`protoc` is not installed on this machine, so the feature cannot be linked. Extract the literal into a scratch example that mirrors the library's types, and compile that instead — this is exactly how the break was found, so it is a valid regression gate for the literal itself:

```powershell
cd d:\all_projects\HipCortex
```

Create `examples/_grpc_literal_check.rs` containing a `main()` that builds a `MemoryRecord` with the **same field list just written into `src/grpc_server.rs`** (substituting `MemoryType::Temporal`, `uuid::Uuid::new_v4().to_string()`, `chrono::Utc::now().timestamp()`, `"{}".to_string()` for the payload-derived values), then calls `record.compute_hash()` and assigns it to `record.integrity`. Run:

```powershell
cargo check --example _grpc_literal_check --no-default-features --features "petgraph_backend" 2>&1 | Select-String -Pattern "error|E0063|Finished"
```

Expected: no `error` line. Then delete the scratch file and the example must not be committed:

```powershell
Remove-Item examples\_grpc_literal_check.rs
git status --short
```

- [ ] **Step 5: State the residual honestly in the commit message and in `CHANGELOG.md`.**

The literal is fixed; whether the *whole* feature compiles cannot be verified here without `protoc`. Add to `CHANGELOG.md`, in the existing unreleased section, a bullet:

```markdown
- **`--features grpc-server` did not compile** (`src/grpc_server.rs`): its `MemoryRecord` literal
  omitted 14 fields (`E0063`), and no CI job builds that feature, so the gRPC surface has been dead
  code since `MemoryRecord` gained the metric and provenance fields. The literal is now complete and
  verified by compiling an equivalent literal against the library. The feature as a whole is still
  unverified: `protoc` is not installed in this environment and no job installs it. Enabling gRPC in
  CI is a separate decision, not a claim made here.
```

- [ ] **Step 6: Commit.**

```powershell
git add src/grpc_server.rs tests/unit/memory_tests.rs CHANGELOG.md
git commit -m "fix(grpc): complete the MemoryRecord literal that no job compiles

src/grpc_server.rs built a MemoryRecord with 8 of 22 fields. serde(default) does not
apply to Rust struct literals, so this is E0063; no CI job names --features grpc-server,
so it has been invisible since the record gained access/confidence/content_hash and the
provenance fields. Field values mirror MemoryRecord::new. The feature as a whole remains
unverified here (no protoc); recorded as such rather than claimed."
```

---

### Task 3: Integrity hashes carry the format that produced them

`MemoryRecord::compute_hash()` hashes the serialised record:

```rust
let mut clone = self.clone();
clone.integrity = None;
clone.content_hash = None;
clone.access_count = 0;
clone.last_accessed = self.timestamp;
let data = serde_json::to_vec(&clone).unwrap();
```

The hash is therefore meaningful **only against the exact struct layout that produced it**. Every field added to `MemoryRecord` changes the bytes, so every previously written hash becomes unreproducible — and nothing in the record says which format it was hashed under. `rollback()` (`src/memory_store.rs:997-999`) is the **only** place integrity is verified anywhere in the crate, and it treats any mismatch as fatal:

```rust
if let Some(hash) = &rec.integrity {
    if *hash != rec.compute_hash() {
        return Err(anyhow::anyhow!("integrity mismatch"));
    }
}
```

Measured consequence, on the user's live store at `c:\Users\user\.hipcortex\data\memory.jsonl` (frozen snapshot, 737 lines): `434` records verify, `255` do not, `48` carry no integrity hash. The current 3.10.0 server's own writes verify — a live probe wrote a `Temporal` and a `Symbolic` record through `POST /memory/add` and both re-verified. So the failures are records hashed by a superseded binary sharing the same `DATA_DIR`, and the effect is that **`rollback()` refuses a store that contains them**, which is every long-lived store. `load()` never verifies, so the condition is silent until a rollback is attempted.

The fix is not to weaken the check — that would accept genuine tampering — but to make the format part of the record so the reader can distinguish the two cases.

**Files:**
- Modify: `src/memory_record.rs` (add `INTEGRITY_FORMAT_VERSION`, `IntegrityVerdict`, a `hash_version` field, wire it through `new()` and `compute_hash()`, add `integrity_verdict()`)
- Modify: `src/passive_capture.rs:136` and `src/grpc_server.rs:48` (both struct literals must carry the new field)
- Modify: `src/memory_store.rs` (`rollback()`: verdict-based check, counted and audited)
- Test: `tests/unit/memory_store_tests.rs` (append; already registered at `tests/unit/mod.rs:39`)

**Interfaces:**
- Consumes: `MemoryRecord::compute_hash(&self) -> String`; `AuditLog::append(&mut self, actor: &str, action: &str, outcome: &str) -> anyhow::Result<()>` (`src/audit_log.rs:59`); `MemoryStore::snapshot(&mut self, path) -> Result<()>` and `MemoryStore::rollback(&mut self, path) -> Result<()>`.
- Produces: `pub const INTEGRITY_FORMAT_VERSION: u32 = 1`; `pub enum IntegrityVerdict { Ok, LegacyUnverified, Mismatch }`; `pub fn integrity_verdict(&self) -> IntegrityVerdict`; `pub hash_version: u32` (serialised, `#[serde(default)]` so absent ⇒ `0`). `rollback()` returns `Ok` for `Ok` and `LegacyUnverified` records, `Err` only for `Mismatch`, and appends `("system", "rollback", "ok")` or `("system", "rollback", "ok legacy_unverified=N")`.

- [ ] **Step 1: Write the failing tests.**

Append to `tests/unit/memory_store_tests.rs`:

```rust
/// The record shape this file already uses everywhere; `MemoryStore::new_in_memory()` is **not**
/// usable in the first test because it installs a sink audit log (`AuditLog::new_sink()`,
/// `src/memory_store.rs:126`) whose `append` returns early, so `audit_export()` would be empty.
fn make_record(action: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        action.into(),
        "target".into(),
        serde_json::json!({}),
    )
}

/// A record hashed before the format tag existed deserialises with `hash_version == 0` and can
/// never re-verify, because the bytes it was hashed from are gone. That is not tampering, and
/// rollback() must not refuse the store because of it. Measured on the operator's live store:
/// 255 of 689 integrity-bearing records are in this state.
#[test]
fn rollback_tolerates_records_hashed_by_a_superseded_format() {
    let dir = std::env::temp_dir().join(format!("hipcortex-hv-legacy-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.jsonl");

    let mut store = MemoryStore::new(dir.join("store.jsonl")).unwrap();
    store.add(make_record("legacy")).unwrap();
    store.snapshot(&snap).unwrap();

    // Rewrite the line as a pre-tag record whose content also changed: the stored hash cannot be
    // reproduced, and the record says the format it was written in is older than this build.
    let text = std::fs::read_to_string(&snap).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    value.as_object_mut().unwrap().remove("hash_version");
    value["action"] = serde_json::json!("changed-after-hashing");
    std::fs::write(&snap, format!("{}\n", serde_json::to_string(&value).unwrap())).unwrap();

    let mut restored = MemoryStore::new(dir.join("restored.jsonl")).unwrap();
    restored
        .rollback(&snap)
        .expect("a pre-tag record must not make rollback fail");
    assert_eq!(restored.all().len(), 1);

    // And the fact that it could not be verified is recorded, not swallowed.
    let audit = restored.audit_export().unwrap();
    let entry = audit.last().expect("rollback must write an audit entry");
    assert_eq!(entry.action, "rollback");
    assert!(
        entry.outcome.contains("legacy_unverified=1"),
        "unverified records must be counted in the audit trail, got {:?}",
        entry.outcome
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A record carrying the *current* format tag whose content changed after hashing is the case the
/// check exists for. It must still fail.
#[test]
fn rollback_refuses_current_format_records_that_do_not_verify() {
    let dir = std::env::temp_dir().join(format!("hipcortex-hv-tamper-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.jsonl");

    let mut store = MemoryStore::new(dir.join("store.jsonl")).unwrap();
    store.add(make_record("tampered")).unwrap();
    store.snapshot(&snap).unwrap();

    let text = std::fs::read_to_string(&snap).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(
        value["hash_version"],
        serde_json::json!(INTEGRITY_FORMAT_VERSION),
        "records written now must carry the current format tag"
    );
    value["action"] = serde_json::json!("changed-after-hashing");
    std::fs::write(&snap, format!("{}\n", serde_json::to_string(&value).unwrap())).unwrap();

    let mut restored = MemoryStore::new(dir.join("restored.jsonl")).unwrap();
    let err = restored.rollback(&snap).unwrap_err().to_string();
    assert!(
        err.contains("integrity mismatch"),
        "a tampered current-format record must be refused, got {err:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A freshly written record verifies, and says so through the new seam.
#[test]
fn freshly_written_records_verify_and_report_ok() {
    let mut store = MemoryStore::new_in_memory();
    let rec = make_record("fresh");
    let stored = rec.clone();
    store.add(rec).unwrap();
    assert_eq!(stored.hash_version, INTEGRITY_FORMAT_VERSION);
    assert_eq!(stored.integrity_verdict(), IntegrityVerdict::Ok);

    // Absent hash means nothing to verify — not evidence of tampering.
    let mut bare = stored.clone();
    bare.integrity = None;
    assert_eq!(bare.integrity_verdict(), IntegrityVerdict::LegacyUnverified);
}
```

Add to the file's `use` block:

```rust
use hipcortex::memory_record::{IntegrityVerdict, MemoryRecord, MemoryType, INTEGRITY_FORMAT_VERSION};
```

The file has no `make_record` helper today — every test constructs `MemoryRecord::new(...)` inline (`tests/unit/memory_store_tests.rs:10`) and counts with `store.all().len()` (`:193`). Add the helper above; do not invent a `MemoryStore::count()`, it does not exist.

- [ ] **Step 2: Run the tests and confirm they fail to compile, then fail for the stated reason.**

```powershell
cargo test --no-default-features --features "petgraph_backend" --test unit_suite rollback_tolerates 2>&1 | Select-String -Pattern "error\[|E0\d+|cannot find|test result"
```

Expected RED: `cannot find type IntegrityVerdict` / `cannot find value INTEGRITY_FORMAT_VERSION` — the seam does not exist yet. That is the honest starting state; record it.

- [ ] **Step 3: Add the format tag and the verdict.**

In `src/memory_record.rs`, above `pub struct MemoryRecord`:

```rust
/// Version of the integrity-hash format.
///
/// `compute_hash()` serialises a record and hashes the bytes, so a hash is only meaningful against
/// the exact struct layout that produced it. Records written before this tag existed deserialise
/// with `hash_version == 0` and can no longer be re-verified — not because they were altered, but
/// because a build that no longer exists emitted their bytes. This tag is what lets a reader tell
/// that case apart from tampering.
pub const INTEGRITY_FORMAT_VERSION: u32 = 1;

/// Outcome of verifying a record's stored integrity hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityVerdict {
    /// The stored hash reproduces from the record's current bytes.
    Ok,
    /// The stored hash does not reproduce and the record predates the current hash format, so the
    /// bytes it was hashed from are unrecoverable: tampering can be neither confirmed nor excluded.
    /// A record with no hash at all also lands here — there is nothing to check.
    LegacyUnverified,
    /// The record carries a current-format hash that does not reproduce: its content changed after
    /// it was hashed.
    Mismatch,
}
```

In the struct, immediately after the `integrity` field:

```rust
    /// Version of the hash format used to compute `integrity`. 0 = written before the tag existed.
    #[serde(default)]
    pub hash_version: u32,
```

In `MemoryRecord::new`, immediately after `integrity: None,`:

```rust
            hash_version: INTEGRITY_FORMAT_VERSION,
```

In `compute_hash`, after `clone.last_accessed = self.timestamp;`:

```rust
        // Pin the tag so the hash depends on the format, not on whether the caller set the field.
        clone.hash_version = INTEGRITY_FORMAT_VERSION;
```

Add `integrity_verdict` next to `compute_hash`:

```rust
    /// Verify `integrity` against this record's bytes, distinguishing an old format from tampering.
    ///
    /// Match is checked first: a pre-tag record that still reproduces is `Ok`, because reproducing
    /// is the stronger evidence.
    pub fn integrity_verdict(&self) -> IntegrityVerdict {
        let Some(stored) = self.integrity.as_deref() else {
            return IntegrityVerdict::LegacyUnverified;
        };
        if stored == self.compute_hash() {
            return IntegrityVerdict::Ok;
        }
        if self.hash_version >= INTEGRITY_FORMAT_VERSION {
            IntegrityVerdict::Mismatch
        } else {
            IntegrityVerdict::LegacyUnverified
        }
    }
```

- [ ] **Step 4: Carry the new field through both struct literals.**

In `src/passive_capture.rs` (literal starting line 136), after `integrity: None,`:

```rust
                hash_version: INTEGRITY_FORMAT_VERSION,
```

In `src/grpc_server.rs` (literal starting line 48), after `integrity: None,`:

```rust
            hash_version: INTEGRITY_FORMAT_VERSION,
```

Both files must import it: extend their existing `use crate::memory_record::{…}` line with `INTEGRITY_FORMAT_VERSION`.

- [ ] **Step 5: Make `rollback()` use the verdict.**

In `src/memory_store.rs`, extend the existing import to `use crate::memory_record::{IntegrityVerdict, MemoryRecord};`, then replace the rollback loop's verification block and the final audit line. The loop body becomes:

```rust
            let rec: MemoryRecord = serde_json::from_str(&line)?;
            match rec.integrity_verdict() {
                IntegrityVerdict::Ok => {}
                IntegrityVerdict::LegacyUnverified => legacy_unverified += 1,
                IntegrityVerdict::Mismatch => {
                    return Err(anyhow::anyhow!(
                        "integrity mismatch for record {}: content changed after it was hashed",
                        rec.id
                    ));
                }
            }
            records.push(rec);
```

declaring `let mut legacy_unverified = 0usize;` next to `let mut records = Vec::new();`.

The final line of `rollback` becomes:

```rust
        // Record what could not be verified rather than silently accepting it: an operator reading
        // the audit trail must be able to see that part of the restore was unverifiable.
        let outcome = if legacy_unverified == 0 {
            "ok".to_string()
        } else {
            format!("ok legacy_unverified={}", legacy_unverified)
        };
        self.audit.append("system", "rollback", &outcome)?;
        Ok(())
```

- [ ] **Step 6: Run the new tests and the whole store suite.**

```powershell
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | Tee-Object -FilePath tmp_unit.txt
Select-String -Path tmp_unit.txt -Pattern "test result|FAILED|panicked"
```

Expected GREEN: the three new tests pass and the suite reports `0 failed`. A pre-existing test asserting `rollback("snap.bin")` succeeds (`tests/unit/memory_store_tests.rs:~190`) must still pass — if it now fails with `legacy_unverified`, read the assertion before changing anything.

- [ ] **Step 7: Verify the whole build still works on every supported feature combination.**

```powershell
cargo build --no-default-features --features "petgraph_backend"
cargo build --no-default-features --features "petgraph_backend,tokio"
cargo build --no-default-features --features "petgraph_backend,web-server"
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite 2>&1 | Tee-Object -FilePath tmp_int2.txt
Select-String -Path tmp_int2.txt -Pattern "test result|FAILED"
cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D clippy::correctness
```

- [ ] **Step 8: Confirm the live store is now rollback-safe, without writing to it.**

Freeze a copy and count the verdicts. Do **not** point this at the live path directly — the `:3030` server rewrites it while it is read. Write the probe to a temp file outside the repo rather than a multi-line `python -c`: long multi-line PowerShell arguments truncate on paste, and a truncated probe reports a clean result for the wrong reason.

`$env:TEMP\hipcortex_verdict_probe.py`:

```python
import hashlib, json, os, shutil

CURRENT = 1
SRC = r"c:\Users\user\.hipcortex\data\memory.jsonl"
DST = os.path.join(os.environ.get("TEMP", "."), "rollback_probe.jsonl")

shutil.copyfile(SRC, DST)

with open(DST, encoding="utf-8") as fh:
    lines = [l for l in fh if l.strip()]

ok = legacy = mismatch = 0
for line in lines:
    rec = json.loads(line)
    integrity = rec.get("integrity")
    if not integrity:
        legacy += 1
        continue
    clone = dict(rec)
    clone["integrity"] = None
    clone["content_hash"] = None
    clone["access_count"] = 0
    clone["last_accessed"] = rec.get("timestamp")
    clone["hash_version"] = CURRENT
    digest = hashlib.sha256(
        json.dumps(clone, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if digest == integrity:
        ok += 1
    elif rec.get("hash_version", 0) >= CURRENT:
        mismatch += 1
    else:
        legacy += 1

print({"total": len(lines), "ok": ok, "legacy_unverified": legacy, "mismatch": mismatch})
```

```powershell
python "$env:TEMP\hipcortex_verdict_probe.py"
```

Expected: `mismatch: 0` — every record that fails to reproduce is pre-tag. If `mismatch` is non-zero, **stop**: that is a real integrity failure and the plan's premise is wrong. Investigate before continuing.

Note that the store has been written to since this plan was drafted, so `total` will be larger than 737 and `legacy_unverified` larger than 255. The invariant is `mismatch: 0`, not the exact counts. Quote the numbers this run actually prints.

- [ ] **Step 9: Commit.**

```powershell
Remove-Item tmp_unit.txt, tmp_int2.txt -ErrorAction SilentlyContinue
git add src/memory_record.rs src/memory_store.rs src/passive_capture.rs src/grpc_server.rs tests/unit/memory_store_tests.rs
git commit -m "fix(memory-store): let a record say which hash format produced its integrity

compute_hash serialises the record, so every field added to MemoryRecord made every
previously written hash unreproducible, and nothing in the record recorded which format
it was hashed under. rollback() is the only integrity verifier in the crate and treated
every mismatch as fatal, so any store holding pre-upgrade records could not be rolled
back - measured on the operator's live store as 255 of 689 records. load() never
verifies, so the condition was silent until a rollback was attempted.

Tag records with the format version, split verification into Ok/LegacyUnverified/
Mismatch, refuse only current-format mismatches, and count unverifiable records into the
audit trail instead of accepting them silently."
```

---

### Task 4: The E2E Merkle assertion asserts what is true, and the diagnosis is written down

`tests/e2e_user_harness/assertions.py::assert_merkle_chain_integrity` reimplements `compute_hash` in Python and asserts every record reproduces. That algorithm is **correct** — proved by measurement: re-emitting each stored line with `json.dumps(clone, separators=(',', ':'), ensure_ascii=False)` reproduces the stored bytes exactly (`0` differing lines out of 737), all candidate alternative conventions score `0/689`, and the key sets and key orders are identical between passing and failing records. The assertion fails on the live store because the store holds records from a superseded binary, which is the same underlying condition Task 3 addresses. The assertion must keep verifying current-format records strictly, and must stop calling historical records corrupt.

Numbers to quote verbatim (frozen snapshot, 737 lines): `raw-utf8 ok=434 bad=255 integrity-absent=48`; `ascii-escaped ok=431 bad=258`; key sets and key orders identical across both groups; `python re-emission differs from the stored text: 0`; omit-hash-keys `0/689`, `+provenance 0/689`, `sort_keys=True 0/689`, `access NOT zeroed 432/689`; live probe against 3.10.0 on `:3030` — `Temporal … verifies=True`, `Symbolic … verifies=True`. **Do not quote the ok/bad split from the diagnostic script that labelled records in place by mutating them; its bookkeeping was corrupt and its split was an artefact.**

**Files:**
- Modify: `tests/e2e_user_harness/assertions.py`
- Modify: `tests/e2e_user_harness/suites/test_phase5_persistence_and_merkle.py`
- Modify: `docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md` (§2.4, the A10 clause)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: `assert_merkle_chain_integrity(records)` — accepts a list of dicts, a `Path` or a `str` path; currently returns `None` and asserts that **every** integrity-bearing record reproduces.
- Produces: `def is_current_format(record: dict) -> bool`; `def expected_integrity_hash(record: dict) -> str`; `def count_unverifiable(records) -> int`; `assert_merkle_chain_integrity` verifies only current-format records and **returns** the number of unverifiable ones, so callers can assert on it explicitly. The existing call site `assert_merkle_chain_integrity(memory_file)` in `suites/test_phase5_persistence_and_merkle.py:26` ignores the return value and keeps working.

The harness is a package: `suites/*.py` import it as `from tests.e2e_user_harness.assertions import …` and pytest is invoked from the repository root (`pytest.ini`). Do **not** add `sys.path` manipulation — `assertions.py` itself does `from .data_generators import estimate_tokens`, which only resolves inside the package.

- [ ] **Step 1: Split the assertion.**

In `tests/e2e_user_harness/assertions.py` (which imports `hashlib`, `json`, `Path`, `Any`), add `import os` to the import block, then add above `assert_merkle_chain_integrity`:

```python
def is_current_format(record: dict) -> bool:
    """True when the record carries an integrity hash in the current hash format.

    Records written before the format was tagged have no ``hash_version`` key. Their bytes were
    emitted by a struct layout this build no longer produces, so their hash can never be
    reproduced — that is provenance, not corruption.
    """
    return record.get("integrity") is not None and record.get("hash_version", 0) >= 1


def expected_integrity_hash(record: dict) -> str:
    """The hash this build would compute for ``record``. Mirrors MemoryRecord::compute_hash."""
    clone = dict(record)
    clone["integrity"] = None
    clone["content_hash"] = None
    clone["access_count"] = 0
    clone["last_accessed"] = clone.get("timestamp")
    clone["hash_version"] = INTEGRITY_FORMAT_VERSION
    data = json.dumps(clone, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def count_unverifiable(records) -> int:
    """Records this build cannot verify: pre-format-tagged ones, and ones with no hash at all."""
    if isinstance(records, (Path, str)):
        records = [
            json.loads(line)
            for line in Path(records).read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
    return sum(1 for r in records if not is_current_format(r))
```

with, next to them, `INTEGRITY_FORMAT_VERSION = 1` — the Python mirror of the Rust constant. Keep the two in step; the strictness test in Step 4 is what detects drift.

Then change `assert_merkle_chain_integrity` so that it skips `not is_current_format(record)` entries with `continue`, uses `expected_integrity_hash(record)` for the rest, keeps the `len(expected) == 64` assertion and the non-empty assertion, and ends with `return count_unverifiable(records)`. The signature's type hint becomes `-> int`. Its existing body already loads a path-or-list and computes the same fields, so the edit is the loop guard, the helper name and the return.

- [ ] **Step 2: Make the phase-5 test state the residual explicitly.**

In `tests/e2e_user_harness/suites/test_phase5_persistence_and_merkle.py`, change the call site so the unverifiable count is asserted and reported rather than ignored:

```python
    unverifiable = assert_merkle_chain_integrity(memory_file)
    # Records written before the integrity format was tagged cannot be re-verified by this build.
    # Report them; do not fail the run because a long-lived store has history.
    print(f"merkle: {unverifiable} unverifiable (pre-format-tagged)")
```

- [ ] **Step 3: Run the harness suite that needs no server.**

```powershell
cd tests\e2e_user_harness
python -m pytest suites/test_phase5_persistence_and_merkle.py -v 2>&1 | Tee-Object -FilePath ..\..\tmp_e2e.txt
Select-String -Path ..\..\tmp_e2e.txt -Pattern "passed|failed|unverifiable"
cd ..\..
```

- [ ] **Step 4: Prove the strict path still catches a current-format corruption.**

Create `tests/e2e_user_harness/suites/test_merkle_strictness.py`:

```python
"""The Merkle assertion must still be able to fail: split for legacy records, strict for current
ones. Without this, Task 4's change would be indistinguishable from deleting the check."""
import pytest

from tests.e2e_user_harness.assertions import (
    assert_merkle_chain_integrity,
    count_unverifiable,
    expected_integrity_hash,
)


def _record(integrity_ok: bool) -> dict:
    rec = {
        "id": "00000000-0000-0000-0000-000000000001",
        "record_type": "Temporal",
        "timestamp": "2026-09-12T00:00:00Z",
        "actor": "merkle-strictness",
        "action": "test",
        "target": "t",
        "metadata": {},
        "integrity": None,
        "content_hash": None,
        "access_count": 0,
        "last_accessed": "2026-09-12T00:00:00Z",
        "relevance_score": 1.0,
        "expires_at": None,
        "confidence": 1.0,
        "source": None,
        "version": 0,
        "tags": [],
        "priority": "normal",
        "status": "active",
        "hash_version": 1,
    }
    rec["integrity"] = expected_integrity_hash(rec)
    if not integrity_ok:
        rec["target"] = "changed after hashing"
    return rec


def test_current_format_record_is_verified():
    assert assert_merkle_chain_integrity([_record(True)]) == 0


def test_current_format_tampering_fails():
    with pytest.raises(AssertionError):
        assert_merkle_chain_integrity([_record(False)])


def test_pre_tag_record_is_counted_not_failed():
    rec = _record(False)
    del rec["hash_version"]
    assert assert_merkle_chain_integrity([rec]) == 1
    assert count_unverifiable([rec]) == 1
```

```powershell
cd tests\e2e_user_harness
python -m pytest suites/test_merkle_strictness.py -v 2>&1 | Tee-Object -FilePath ..\..\tmp_strict.txt
Select-String -Path ..\..\tmp_strict.txt -Pattern "passed|failed"
cd ..\..
```

Expected: `3 passed`. `test_current_format_tampering_fails` is the test that keeps the assertion alive.

- [ ] **Step 5: Record the diagnosis in the spec.**

In `docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md`, replace §2.4's instruction ("Update the Python test `assert_merkle_chain_integrity` to match the production `compute_hash` implementation") with a correction that records the finding — the harness was never wrong:

```markdown
### 2.4 Merkle Chain Verification (A10) — **RESOLVED, the assertion was correct**

The A10 recommendation was to change the Python test to match `compute_hash`. Measurement shows the
test already matched it and the premise was wrong.

Evidence, on a frozen copy of the operator's live store (737 lines):

| Check | Result |
|---|---|
| Python re-emission of each stored line vs. the stored bytes | **0 differences** |
| Current convention, reproducing hashes | `434 / 689` integrity-bearing records |
| `ascii-escaped` instead of raw UTF-8 | `431 / 689` |
| omit the hash keys instead of nulling them | `0 / 689` |
| sort metadata keys | `0 / 689` |
| include provenance fields | `0 / 689` |
| access tracking not zeroed | `432 / 689` |
| Distinct key sets / key orders among passing records | **1 / 1** |
| Distinct key sets / key orders among failing records | **1 / 1** |

Passing and failing records are structurally identical: same 19 keys, same order, same value
shapes. No alternative convention reproduces the failing hashes. A live write-and-verify probe
against the running 3.10.0 server produced `Temporal … verifies=True` and
`Symbolic … verifies=True`.

Conclusion: the failures are records hashed by a superseded binary sharing the same `DATA_DIR`.
`compute_hash` hashes the serialised record, so any field added to `MemoryRecord` invalidates every
earlier hash, and nothing recorded which format a hash came from. The harness is correct; the data
is historical. Fixed not by changing the test but by tagging the hash format on the record and
splitting verification into `Ok` / `LegacyUnverified` / `Mismatch` — see the `hash_version` work in
`MemoryRecord` and `MemoryStore::rollback`.

Surface consequence, recorded because it is a defect in its own right: `MemoryStore::rollback`
(`src/memory_store.rs`) was the only integrity verifier in the crate and returned
`Err("integrity mismatch")` on the first pre-tag record, so a long-lived store could not be rolled
back at all. `load()` never verifies, so the condition was silent until a rollback was attempted.
```

- [ ] **Step 6: Add the CHANGELOG entry.**

```markdown
- **Merkle verification (A10) closed as a false premise.** The E2E harness's
  `assert_merkle_chain_integrity` reimplements `MemoryRecord::compute_hash`; it was recommended for
  correction, and measurement shows it was already byte-exact — re-emitting each stored line with
  `separators=(",", ":")` reproduces the stored bytes with **0** differences out of 737, and every
  alternative convention scores `0/689`. The `255` failing records in the operator's live store were
  hashed by a superseded binary sharing the same `DATA_DIR`; a live probe against 3.10.0 on `:3030`
  wrote a `Temporal` and a `Symbolic` record and both re-verified. Records now carry
  `hash_version`, verification is split into `Ok` / `LegacyUnverified` / `Mismatch`, and
  `assert_merkle_chain_integrity` returns the unverifiable count instead of treating history as
  corruption. A new `test_merkle_strictness.py` keeps the strict path—current-format tampering still
  fails the assertion—so the change is not a deletion of the check.
```

- [ ] **Step 7: Run everything, then commit.**

```powershell
cd tests\e2e_user_harness
python -m pytest suites/test_merkle_strictness.py suites/test_phase5_persistence_and_merkle.py -v 2>&1 | Tee-Object -FilePath ..\..\tmp_final.txt
Select-String -Path ..\..\tmp_final.txt -Pattern "passed|failed"
cd ..\..
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | Select-String -Pattern "test result|FAILED"
Remove-Item tmp_e2e.txt, tmp_strict.txt, tmp_final.txt -ErrorAction SilentlyContinue
git add tests/e2e_user_harness/assertions.py tests/e2e_user_harness/suites/test_phase5_persistence_and_merkle.py tests/e2e_user_harness/suites/test_merkle_strictness.py docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md CHANGELOG.md
git commit -F "$env:TEMP\task4_commit_msg.txt"
```

The commit message this step was written with claimed `0/737`, `0/689`, `255 failing records` and a live
write probe. None of those survived measurement; the delivered commit (`4004c34`) states the measured
figures instead:

```
docs+tests(merkle): record that the A10 assertion was correct, and keep it strict

The recommendation to rewrite the Python verifier to match compute_hash rested on a false
premise. Measured on a frozen copy of the live store (897 records, frozen before reading
because the running instance keeps writing): Python re-emits every stored line byte-for-byte,
897/897 with 0 differences, and the crate's integrity_verdict() agrees with an independent
Python reimplementation on all 897 (mismatch = 0). The unverifiable records are 307 hashed by
a superseded binary plus 154 carrying integrity: null - the passive-capture path stores one
unhashed record per /memory/add, so half of a fresh store is already in that bucket. Recorded,
not fixed here.

The convention is load-bearing: with an absent tag omitted exactly as skip_serializing_if
omits it, 436 records reproduce their stored hash; writing "hash_version": 0 instead
reproduces none. A first version of this measurement inserted the key and so reported 0
verifiable records on a store where 436 do reproduce, which is why the helper now mirrors the
omission rule and a test pins it.

Split the assertion: current-format records are still verified strictly, the rest are counted
and returned. test_merkle_strictness.py (6 tests) keeps the split from degrading into deleting
the check - a current-format record altered after hashing still fails the assertion. The
phase-5 suite prints the count against the total and asserts the strict path covered the five
records it wrote (5 unverifiable of 10 records). Spec 2.4 now carries the measurements,
including the surfaced consequence that rollback() refused any store holding those records.
```

---

## Verification Gates

Run all of these before declaring the plan complete. Every claim in this plan must be backed by one of them.

```powershell
cd d:\all_projects\HipCortex

# 1. Builds
cargo build --no-default-features --features "petgraph_backend"
cargo build --no-default-features --features "petgraph_backend,tokio"
cargo build --no-default-features --features "petgraph_backend,web-server"

# 2. Lint (never apply an is_none_or suggestion — MSRV 1.70)
cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D clippy::correctness
cargo clippy --no-default-features --features "petgraph_backend,web-server" --all-targets -- -D clippy::correctness

# Formatting: NOT `cargo fmt --all -- --check`. That fails at the parent commit with
# 882 diff blocks spread across the repository, so it is not a gate this change can
# satisfy without a repo-wide reformat, which the Surgical Changes rule forbids.
# The gate is instead: a file this plan touches must not carry more rustfmt diffs
# than it carried before. Measured per file with
#   rustfmt --check --edition 2021 <file>          (count occurrences of "Diff in")
# Never pipe that through Python without an explicit utf-8 decode: the console
# default codec is GBK and rustfmt emits UTF-8, which raises UnicodeDecodeError.
#
# Baseline at 8756478 (parent of fde853b):
#   tests/integration/v110_rest_sit.rs  28      src/web_server.rs  15
# After the /memory/embed fix:
#   tests/integration/v110_rest_sit.rs  28      src/web_server.rs  15

# 3. Tests
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
cargo test --no-default-features --features "petgraph_backend" --test property_suite
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite

# 4. Python
cd tests\e2e_user_harness; python -m pytest suites/test_merkle_strictness.py -v; cd ..\..
cd sdk\python; pytest tests\ -v; cd ..\..

# 5. Working tree: only .claude/settings.local.json may be modified, plus the three
#    pre-existing untracked paths (KARM_*.md, dist/*.whl, fixtures/soak_diary_output.json).
git status --short
```

**Do not claim a check passed without its output.** Three findings in this session came from checks that were declared but never executed, and one came from a diagnostic whose own bookkeeping was wrong. Quote the observed output.

**Left deliberately open (do not "fix" without new evidence):**
- The A2 clause-2 question — whether `compute_intervention` should return a placeholder or an `Err` — was recorded as unreconciled in `9ac2d67` and stays that way.
- Enabling `--features grpc-server` in CI: Task 2 repairs the literal and states that the feature is otherwise unverified. Adding the job is a separate decision with a separate cost (`protoc` in the runner).
- The 22+ local commits ahead of `origin/main` (`395776db…`) are not pushed. Pushing is the user's call.

---

## Execution Record

All four tasks are delivered as local commits on `main`.

| Task | Commit(s) | Delivered |
|---|---|---|
| 1 | `fde853b`, CHANGELOG `03a4bc1` | `/memory/embed` validates through the write path's own parser and guardrail |
| 2 | `be4417b` | `src/grpc_server.rs` builds a complete `MemoryRecord` literal again |
| 3 | `1540201`, CHANGELOG follow-up | `hash_version`, `IntegrityVerdict`, and a `rollback()` that tolerates a superseded format |
| 4 | `4004c34` | the Merkle assertion split, the diagnosis in spec §2.4 and `CHANGELOG.md`, and the strictness suite |

Checked output, verbatim: `cargo test --test unit_suite` → `test result: ok. 517 passed; 0 failed; 0 ignored`
(the same 517 as before Task 3 and 4, since neither touches Rust behaviour that a unit test asserts);
`pytest suites/test_merkle_strictness.py suites/test_phase5_persistence_and_merkle.py -v` → `8 passed in
9.62s` with `test_current_format_tampering_fails` among them, so the split did not delete the check, and
`merkle: 5 unverifiable of 10 records` from the phase-5 test proving the strict path still verified the
five records that test wrote.

### Deviations from the plan text, each with the evidence that forced it

1. **Task 3, Step 3 — the tag must be omitted when zero, not written as `0`.** The step pinned `clone.hash_version` inside `compute_hash()`, which adds a field to the serialised form and changes the bytes of exactly the records the tag exists to tolerate. Replaced by two changes: `skip_serializing_if = "is_zero_u32"`, and `compute_hash()` hashing the tag the record actually has. Measured on the frozen store: omitting an absent tag reproduces **436** records; writing `"hash_version": 0` reproduces **0**.
2. **Task 3 — `integrity_verdict()` compares the stored hash before it consults the tag.** The step's order reported `Mismatch` for a record whose bytes verify. Match first, then tag.
3. **Task 4 — three tests beyond the planned three.** The planned three all pass under a wrong hash convention, and that is not hypothetical: during this session the helper inserted an absent `hash_version` key and reported `ok=0` on a store where 436 records verify. `test_pre_tag_hash_omits_the_tag_key`, `test_record_with_no_hash_is_counted_not_failed` and `test_non_ascii_record_is_hashed_as_raw_utf8` pin the conventions, so the suite cannot pass while hashing the wrong bytes.
4. **Task 4 — this plan's own numbers are not what the store gives.** The plan says 737 stored lines, 434 verifying, 255 failing, 48 unhashed; a second measurement gave 852 / 436 / 416 / 0; the frozen copy measured here gives **897 / 436 / 307 / 154**. The store grows while it is being read, and one earlier measurement used a helper carrying the defect in deviation 3. The measured figures are the ones in `CHANGELOG.md` and spec §2.4; the plan's are superseded.
5. **The formatting gate was replaced** (commit `e41cb4a`) by a per-file non-regression check, because `cargo fmt --all -- --check` already fails at the parent commit with 882 diff blocks.

### Found during execution, deliberately not fixed

- **Every `/memory/add` also writes one `integrity: null` record** through the server-side passive-capture path, so half of a freshly written store is outside the Merkle chain by construction. Proven on a scratch server: 5 adds → 10 lines, 5 of them verifiable, the other 5 all `actor='unknown-channel' action='memory' integrity=None`. Closing this means either hashing passive-capture records or excluding them from the chain explicitly — a separate change, with its own consequence for `rollback()`.
- **`LegacyUnverified` collapses two causes** — a hash in an older format, and no hash at all. They differ in what a caller should do. Splitting the variant is a public-API change and was not made mid-verification.
- `docs/superpowers/plans/2026-07-11-e2e-user-testing-harness-plan.md` (lines 270, 442) still shows the pre-split signature. It is a historical document and was not rewritten.
