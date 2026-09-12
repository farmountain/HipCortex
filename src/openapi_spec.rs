//! OpenAPI 3.0 specification — served at GET /openapi.json
//!
//! The document below is hand-authored for narrative quality (summaries,
//! parameter schemas, examples). It is **not** the authority on which routes
//! exist — [`ROUTE_TABLE`] is. [`spec_with_route_table`] merges the two so the
//! served document can never omit a live route, and
//! `tests/integration/route_parity_sit.rs` fails if the two ever disagree.

use std::collections::HashMap;

/// Every `(METHOD, path)` pair registered by
/// [`crate::web_server::build_app`] — the router that is actually served.
///
/// # Why a declared table instead of runtime introspection
///
/// axum 0.6's `Router` exposes no route-table accessor (`route`, `nest`,
/// `merge`, `layer` and friends all return `Self`; nothing enumerates). So the
/// served OpenAPI document cannot be derived from the router itself and the
/// route surface has to be *declared* and then *checked against the source*.
///
/// This is not hypothetical bookkeeping. Before this table existed:
///
/// * the hand-maintained spec documented 39 of 123 live paths, so 84 working
///   endpoints were invisible to every client that generates from
///   `/openapi.json`;
/// * a second router, `run_with_both_stores`, owned 9 more routes that were
///   never reachable from `main` — `POST /goal/:id/clarify` answered **404**
///   in production while `GET /goal/:id/trace` answered 200.
///
/// # Invariants (enforced by `tests/integration/route_parity_sit.rs`)
///
/// 1. **Complete** — parsing the `.route(...)` calls inside `build_app` yields
///    exactly this set. Adding a route without adding it here fails the test.
/// 2. **Sound** — every path here really is registered, so the served document
///    never advertises an endpoint that 404s.
/// 3. **Documented** — `spec_with_route_table()` describes every entry, and the
///    hand-authored spec contains no path that is absent from here.
///
/// Add an entry in the same commit as the `.route(...)` call. Path parameters
/// use axum's `:name` form; the merge compares `:name` and `{name}` as equal.
pub const ROUTE_TABLE: &[(&str, &str)] = &[
    ("GET", "/"),
    ("POST", "/agent/check-progress"),
    ("POST", "/agent/clarify-goal"),
    ("POST", "/agent/plan-validation"),
    ("POST", "/agent/recommend-tools"),
    ("POST", "/agent/should-exit"),
    ("GET", "/audit/export"),
    ("GET", "/audit/verify"),
    ("POST", "/coherence/check"),
    ("GET", "/coherence/inconsistencies"),
    ("POST", "/coherence/resolve/:id"),
    ("GET", "/coherence/status"),
    ("POST", "/decide/batch"),
    ("POST", "/goal/:id/clarify"),
    ("POST", "/goal/:id/react"),
    ("GET", "/goal/:id/trace"),
    ("GET", "/goal/:id/verify"),
    ("GET", "/graph"),
    ("POST", "/graph/edge"),
    ("POST", "/graph/node"),
    ("DELETE", "/graph/node/:id"),
    ("GET", "/graph/search"),
    ("GET", "/health"),
    ("GET", "/health/summary"),
    ("GET", "/intent/open"),
    ("POST", "/intent/open"),
    ("POST", "/intent/receipt"),
    ("DELETE", "/memory/:id"),
    ("POST", "/memory/add"),
    ("POST", "/memory/bulk"),
    ("POST", "/memory/consolidate"),
    ("POST", "/memory/context"),
    ("POST", "/memory/contradict/:id"),
    ("POST", "/memory/corroborate/:id"),
    ("POST", "/memory/diff"),
    ("POST", "/memory/embed"),
    ("GET", "/memory/export"),
    ("DELETE", "/memory/forget/:actor"),
    ("GET", "/memory/hypotheses"),
    ("POST", "/memory/hypotheses/reset"),
    ("POST", "/memory/ingest"),
    ("GET", "/memory/latest"),
    ("POST", "/memory/link"),
    ("GET", "/memory/live_beliefs"),
    ("GET", "/memory/neighbors/:id"),
    ("POST", "/memory/quarantine/:id"),
    ("GET", "/memory/query"),
    ("POST", "/memory/reflect"),
    ("POST", "/memory/restore/:id"),
    ("POST", "/memory/search"),
    ("GET", "/memory/search/related"),
    ("GET", "/memory/search-flat"),
    ("PATCH", "/memory/update/:id"),
    ("GET", "/metrics"),
    ("GET", "/node/:id"),
    ("GET", "/ns"),
    ("GET", "/openapi.json"),
    ("GET", "/predict/entity/:id"),
    ("GET", "/pricing"),
    ("GET", "/regulatory/hold"),
    ("POST", "/regulatory/hold"),
    ("DELETE", "/regulatory/hold/:actor"),
    ("GET", "/self/can-execute"),
    ("GET", "/self/capabilities"),
    ("POST", "/self/capabilities"),
    ("GET", "/self/health"),
    ("GET", "/stats"),
    ("GET", "/substrate/budget"),
    ("GET", "/substrate/scorecard"),
    ("GET", "/tier"),
    ("POST", "/topo/apply-hyp"),
    ("POST", "/topo/check-edge"),
    ("POST", "/topo/deconstruct"),
    ("GET", "/topo/ppr"),
    ("GET", "/v1/actions/authorized"),
    ("GET", "/v1/actions/authorized-wm"),
    ("GET", "/v1/beliefs"),
    ("POST", "/v1/causal/counterfactual"),
    ("POST", "/v1/causal/credit-assign"),
    ("POST", "/v1/causal/intervene"),
    ("POST", "/v1/causal/rewrite-equation"),
    ("GET", "/v1/cognitive/diff"),
    ("GET", "/v1/cognitive/report"),
    ("GET", "/v1/cognitive/snapshot"),
    ("POST", "/v1/cognitive/transact"),
    ("POST", "/v1/experience/:actor/search"),
    ("GET", "/v1/experience/:actor/tiers"),
    ("POST", "/v1/fork"),
    ("DELETE", "/v1/fork/:fork_id"),
    ("POST", "/v1/fork/:fork_id/rollout"),
    ("GET", "/v1/fork/:fork_id/snapshot"),
    ("POST", "/v1/fork/:fork_id/step"),
    ("POST", "/v1/fork/:fork_id/transact"),
    ("GET", "/v1/goals"),
    ("POST", "/v1/loop/omega"),
    ("GET", "/v1/loop/status/:handle"),
    ("POST", "/v1/loop/stop/:handle"),
    ("POST", "/v1/loop/subscribe"),
    ("GET", "/v1/mat"),
    ("GET", "/v1/memory/:id/provenance"),
    ("POST", "/v1/memory/consolidate"),
    ("POST", "/v1/mgv/check"),
    ("GET", "/v1/self/health"),
    ("POST", "/v1/state/diff"),
    ("GET", "/v1/state/export"),
    ("GET", "/v1/state/tx"),
    ("POST", "/v1/twin"),
    ("GET", "/v1/twin/:twin_id"),
    ("POST", "/v1/twin/:twin_id/rollout"),
    ("POST", "/v1/twin/:twin_id/step"),
    ("POST", "/v1/workspace/:id/renew"),
    ("GET", "/webhooks"),
    ("POST", "/webhooks"),
    ("DELETE", "/webhooks/:id"),
    ("GET", "/worldmodel/causal"),
    ("POST", "/worldmodel/causal/counterfactual"),
    ("POST", "/worldmodel/causal/edge"),
    ("POST", "/worldmodel/causal/intervention"),
    ("GET", "/worldmodel/entities"),
    ("POST", "/worldmodel/entity"),
    ("POST", "/worldmodel/observe"),
    ("GET", "/worldmodel/predict"),
    ("POST", "/worldmodel/predict"),
    ("POST", "/worldmodel/rollout"),
    ("GET", "/worldmodel/states"),
    ("GET", "/worldmodel/status"),
    ("GET", "/worldmodel/transitions"),
    ("GET", "/worldmodel/uncertainty"),
];

/// Canonical path form used when comparing router paths against documented
/// paths: both `:id` (axum) and `{id}` (OpenAPI) collapse to `:`.
///
/// Without this, `/memory/forget/:actor` and `/memory/forget/{actor}` look like
/// two different endpoints and every parameterised route reads as undocumented.
pub fn canonical_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                }
                out.push(':');
            }
            ':' => {
                while let Some(&inner) = chars.peek() {
                    if inner.is_ascii_alphanumeric() || inner == '_' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(':');
            }
            other => out.push(other),
        }
    }
    out
}

/// Derive the capability name of a declared route.
///
/// H8: the SelfModel capability registry was a *fourth* hand-maintained list
/// (`bin/webserver.rs`), disjoint from the router. It is now a projection of
/// [`ROUTE_TABLE`], so a route cannot exist without its capability and a
/// capability cannot exist without its route — removing a route removes its
/// capability, adding a route registers it. See [`crate::capability_catalog`]
/// for the single declared authority.
///
/// The mapping is total over `ROUTE_TABLE` and **injective as currently
/// declared** — `tests/integration/capability_catalog_sit.rs` asserts this, so a
/// future route pair that would collapse (`/a/b` vs `/a-b`) fails the guard
/// rather than silently merging two capabilities into one.
///
/// `("POST", "/memory/add")` → `post_memory_add`;
/// `("GET", "/")` → `get`; `("POST", "/goal/:id/react")` → `post_goal_id_react`.
pub fn capability_name(method: &str, path: &str) -> String {
    let mut out = String::with_capacity(method.len() + path.len() + 1);
    out.push_str(&method.to_ascii_lowercase());
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        let trimmed = segment
            .trim_start_matches(':')
            .trim_matches(|c| c == '{' || c == '}');
        let cleaned: String = trimmed
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let cleaned = cleaned.trim_matches('_');
        if !cleaned.is_empty() {
            out.push('_');
            out.push_str(cleaned);
        }
    }
    out
}

/// Every capability implied by the declared route surface — one per
/// [`ROUTE_TABLE`] entry, sorted and deduplicated.
///
/// Duplicates can only arise if two entries derive the same name, which is the
/// collision the injectivity guard exists to catch.
pub fn route_capability_names() -> Vec<String> {
    let mut names: Vec<String> = ROUTE_TABLE
        .iter()
        .map(|(method, path)| capability_name(method, path))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Exact inverse of [`capability_name`] over [`ROUTE_TABLE`] — the route a
/// capability came from, or `None` if no declared route produces that name.
///
/// This is what makes "the 15-name literal is gone" checkable rather than merely
/// asserted: every route-derived capability resolves back to a real route, so a
/// capability with no route is unrepresentable.
pub fn capability_route(name: &str) -> Option<(&'static str, &'static str)> {
    ROUTE_TABLE
        .iter()
        .copied()
        .find(|(method, path)| capability_name(method, path) == name)
}

/// Deterministic `operationId` for a route that the hand-authored spec does not
/// describe. Derived from method + path so regenerating never churns the diff.
fn auto_operation_id(method: &str, path: &str) -> String {
    let mut id = String::with_capacity(path.len() + method.len() + 8);
    id.push_str("auto_");
    id.push_str(&method.to_ascii_lowercase());
    for c in path.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else {
            id.push('_');
        }
    }
    while id.ends_with('_') {
        id.pop();
    }
    id
}

/// [`OPENAPI_SPEC`] parsed as JSON with every [`ROUTE_TABLE`] entry guaranteed
/// to be present in `paths`.
///
/// Hand-authored operations are preserved verbatim; only the gaps are filled.
/// This is what `GET /openapi.json` serves.
pub fn spec_with_route_table() -> serde_json::Value {
    let mut doc: serde_json::Value =
        serde_json::from_str(OPENAPI_SPEC).expect("openapi spec is valid JSON");
    let paths = doc
        .get_mut("paths")
        .and_then(|v| v.as_object_mut())
        .expect("openapi spec has a `paths` object");

    // Index the documented paths by canonical form so `:id` and `{id}` compare equal.
    let mut documented: HashMap<String, String> = HashMap::new();
    for key in paths.keys() {
        documented.insert(canonical_path(key), key.clone());
    }

    for (method, route) in ROUTE_TABLE {
        let verb = method.to_ascii_lowercase();
        let key = match documented.get(&canonical_path(route)) {
            Some(existing) => existing.clone(),
            None => {
                let inserted = (*route).to_string();
                paths.insert(inserted.clone(), serde_json::Value::Object(Default::default()));
                documented.insert(canonical_path(route), inserted.clone());
                inserted
            }
        };
        let entry = paths
            .get_mut(&key)
            .and_then(|v| v.as_object_mut())
            .unwrap_or_else(|| panic!("openapi paths entry {key} is an object"));
        if entry.contains_key(&verb) {
            continue;
        }
        entry.insert(
            verb,
            serde_json::json!({
                "tags": ["auto-registered"],
                "summary": "Registered in `web_server::build_app`; declared in `openapi_spec::ROUTE_TABLE`.",
                "operationId": auto_operation_id(method, route),
                "responses": { "200": { "description": "OK" } }
            }),
        );
    }

    doc
}

pub const OPENAPI_SPEC: &str = r##"{
  "openapi": "3.0.3",
  "info": {
    "title": "HipCortex Memory Engine",
    "description": "Persistent causal memory for AI agents. Sub-millisecond writes, temporal decay, causal world model, GDPR-compliant.",
    "version": "3.11.0",
    "license": { "name": "Apache 2.0", "url": "https://opensource.org/licenses/Apache-2.0" },
    "contact": { "url": "https://github.com/farmountain/HipCortex" }
  },
  "servers": [
    { "url": "https://hipcortex.fly.dev", "description": "Managed SaaS (Frankfurt)" },
    { "url": "http://127.0.0.1:3030", "description": "Local self-hosted" }
  ],
  "security": [{ "ApiKeyAuth": [] }],
  "components": {
    "securitySchemes": {
      "ApiKeyAuth": {
        "type": "apiKey", "in": "header", "name": "X-Api-Key",
        "description": "Required when HIPCORTEX_API_KEYS is set. Unset = open mode."
      }
    },
    "schemas": {
      "AddMemoryRequest": {
        "type": "object",
        "required": ["actor", "action", "target"],
        "properties": {
          "actor": { "type": "string", "example": "user-42" },
          "action": { "type": "string", "example": "said" },
          "target": { "type": "string", "example": "The meeting is at 3pm" },
          "record_type": { "type": "string", "default": "Temporal",
            "enum": ["Temporal","Symbolic","Procedural","Reflexion","Perception"] },
          "metadata": { "type": "object" },
          "ttl_seconds": { "type": "integer", "nullable": true,
            "description": "Auto-expire after N seconds" },
          "confidence": { "type": "number", "minimum": 0, "maximum": 1,
            "description": "Reliability signal [0.0, 1.0]. Default 1.0." },
          "source": { "type": "string", "nullable": true,
            "description": "Who/what wrote this memory (e.g. 'user-input', 'claude-3-7')" }
        }
      },
      "MemoryRecord": {
        "type": "object",
        "properties": {
          "id": { "type": "string", "format": "uuid" },
          "record_type": { "type": "string" },
          "timestamp": { "type": "string", "format": "date-time" },
          "actor": { "type": "string" },
          "action": { "type": "string" },
          "target": { "type": "string" },
          "metadata": { "type": "object" },
          "integrity": { "type": "string", "nullable": true }
        }
      },
      "BulkAddRequest": {
        "type": "object",
        "required": ["records"],
        "properties": {
          "records": { "type": "array",
            "items": { "$ref": "#/components/schemas/AddMemoryRequest" } }
        }
      },
      "SearchRequest": {
        "type": "object",
        "required": ["query"],
        "properties": {
          "query": { "type": "string" },
          "embedding": { "type": "array", "items": { "type": "number" }, "nullable": true },
          "limit": { "type": "integer", "default": 10 }
        }
      },
      "EmbedAndAddRequest": {
        "type": "object",
        "required": ["actor", "action", "target", "embedding_model"],
        "properties": {
          "actor": { "type": "string" },
          "action": { "type": "string" },
          "target": { "type": "string" },
          "record_type": { "type": "string", "default": "Temporal" },
          "metadata": { "type": "object" },
          "embedding_model": { "type": "string",
            "example": "ollama/nomic-embed-text",
            "description": "Format: ollama/<model> or openai/<model>" }
        }
      }
    }
  },
  "paths": {
    "/health": { "get": { "operationId": "healthCheck", "summary": "Health check", "security": [],
      "responses": { "200": { "description": "ok" } } } },
    "/stats": { "get": { "operationId": "getStats", "summary": "Live server statistics (public)", "security": [],
      "responses": { "200": { "description": "Stats JSON" } } } },
    "/pricing": { "get": { "operationId": "getPricing", "summary": "Pricing page HTML", "security": [],
      "responses": { "200": { "description": "HTML" } } } },
    "/openapi.json": { "get": { "operationId": "getOpenApiSpec", "summary": "This OpenAPI spec", "security": [],
      "responses": { "200": { "description": "OpenAPI 3.0 JSON" } } } },
    "/memory/add": { "post": { "operationId": "addMemory", "summary": "Store a memory record",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/AddMemoryRequest" } } } },
      "responses": { "200": { "description": "Success" } } } },
    "/memory/bulk": { "post": { "operationId": "bulkAddMemory", "summary": "Store multiple records in one request",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/BulkAddRequest" } } } },
      "responses": { "200": { "description": "Bulk result" } } } },
    "/memory/query": { "get": { "operationId": "queryMemory", "summary": "Query memory records",
      "parameters": [
        { "name": "actor", "in": "query", "schema": { "type": "string" } },
        { "name": "action", "in": "query", "schema": { "type": "string" } },
        { "name": "record_type", "in": "query", "schema": { "type": "string" } },
        { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 100 } }
      ],
      "responses": { "200": { "description": "Records" } } } },
    "/memory/search": { "post": { "operationId": "searchMemory", "summary": "Semantic + keyword search. Add embedding_model to auto-generate query embedding.",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/SearchRequest" } } } },
      "responses": { "200": { "description": "Search results" } } } },
    "/memory/search/related": { "get": { "operationId": "searchRelated",
      "summary": "Find memories related to a seed by Personalized PageRank (PPR) over the CausalTopoGraph.",
      "description": "Scores reflect graph centrality from the seed node (topological proximity via PPR, alpha=0.85, 20 iterations). This is independent of record priority: the pinned=2.0 score override used by search_semantic does NOT apply. Pinned records appear only if topologically reachable from the seed via explicit /memory/link edges. A pinned record's prominence here is determined by its graph in-degree, not its priority label.",
      "parameters": [
        { "name": "seed_id", "in": "query", "required": true, "schema": { "type": "string", "format": "uuid" }, "description": "UUID of the seed memory record" },
        { "name": "limit",   "in": "query", "schema": { "type": "integer", "default": 10, "maximum": 50 }, "description": "Max number of results" }
      ],
      "responses": { "200": { "description": "PPR-ranked related records" } } } },
    "/memory/embed": { "post": { "operationId": "embedAndStore", "summary": "Auto-generate embedding then store memory",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/EmbedAndAddRequest" } } } },
      "responses": { "200": { "description": "Stored with embedding" } } } },
    "/memory/export": { "get": { "operationId": "exportMemory", "summary": "Export all memory records as JSON (data portability)",
      "parameters": [
        { "name": "actor", "in": "query", "schema": { "type": "string" }, "description": "Filter by actor (optional)" }
      ],
      "responses": { "200": { "description": "All records as JSON array" } } } },
    "/memory/forget/{actor}": { "delete": { "operationId": "forgetActor", "summary": "GDPR right-to-forget — delete all records for actor",
      "parameters": [{ "name": "actor", "in": "path", "required": true,
        "schema": { "type": "string" } }],
      "responses": { "200": { "description": "Deleted" } } } },
    "/coherence/status": { "get": { "operationId": "getCoherenceStatus", "summary": "Cross-module coherence metrics",
      "responses": { "200": { "description": "Coherence state" } } } },
    "/tier": { "get": { "operationId": "getTier", "summary": "API key tier info and limits",
      "responses": { "200": { "description": "Tier + limits" } } } },
    "/graph": { "get": { "operationId": "getGraph", "summary": "Full symbolic knowledge graph",
      "security": [],
      "responses": { "200": { "description": "Nodes and edges" } } } },
    "/memory/update/{id}": { "patch": { "operationId": "updateMemory",
      "summary": "Versioned in-place update of a memory record",
      "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "type": "object", "properties": {
          "target": { "type": "string" },
          "action": { "type": "string" },
          "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
          "source": { "type": "string" },
          "metadata": { "type": "object" }
        }}}}},
      "responses": { "200": { "description": "Updated" }, "404": { "description": "Not found" } } } },
    "/memory/latest": { "get": { "operationId": "getLatestMemory",
      "summary": "Most recent unique fact per actor+action (solves current-value queries)",
      "security": [],
      "parameters": [
        { "name": "actor", "in": "query", "schema": { "type": "string" } },
        { "name": "action", "in": "query", "schema": { "type": "string" } },
        { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 20 } }
      ],
      "responses": { "200": { "description": "Latest records" } } } },
    "/memory/ingest": { "post": { "operationId": "ingestMemory",
      "summary": "Zero-config smart ingest — auto-classifies record_type, priority, TTL, tags from plain text",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "type": "object", "required": ["text"], "properties": {
          "text": { "type": "string", "description": "Plain text to remember" },
          "actor": { "type": "string", "description": "Optional actor override" },
          "session_id": { "type": "string" },
          "context": { "type": "string", "enum": ["meeting", "code", "chat", "sensor", "decision"] }
        }}}}},
      "responses": { "200": { "description": "Stored with auto-classification result" } } } },
    "/worldmodel/predict": {
      "post": {
        "operationId": "predictNextState",
        "summary": "Predict next state using the world-model's transition model",
        "security": [],
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "type": "object",
                "required": ["state", "action"],
                "properties": {
                  "state": { "type": "string" },
                  "action": { "type": "string" }
                }
              }
            }
          }
        },
        "responses": { "200": { "description": "Prediction result" } }
      },
      "get": {
        "operationId": "predictNextStateGet",
        "summary": "Predict next state via query params (GET)",
        "security": [],
        "parameters": [
          { "name": "state", "in": "query", "required": true, "schema": { "type": "string" } },
          { "name": "action", "in": "query", "required": true, "schema": { "type": "string" } }
        ],
        "responses": { "200": { "description": "Prediction result" } }
      }
    },
    "/worldmodel/rollout": {
      "post": {
        "operationId": "worldModelRollout",
        "summary": "Multi-step rollout: dirichlet MAP (default), MCTS (mode=mcts), or ensemble predictors",
        "security": [],
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "type": "object",
                "required": ["initial_state"],
                "properties": {
                  "initial_state": { "type": "string" },
                  "actions": { "type": "array", "items": { "type": "string" }, "description": "Required unless mode=mcts" },
                  "mode": { "type": "string", "enum": ["dirichlet", "mcts", "ensemble"], "default": "dirichlet" },
                  "iterations": { "type": "integer", "default": 50, "description": "MCTS iterations" },
                  "max_depth": { "type": "integer", "default": 3, "description": "MCTS / trajectory depth" },
                  "goal_state": { "type": "string", "description": "Goal-shaped MCTS reward target state" }
                }
              }
            }
          }
        },
        "responses": { "200": { "description": "Rollout prediction sequence" } }
      }
    },
    "/self/can-execute": {
      "get": {
        "operationId": "selfCanExecute",
        "summary": "Query SelfModel decision engine if operation is safe to execute",
        "security": [],
        "parameters": [
          { "name": "operation", "in": "query", "required": true, "schema": { "type": "string" } }
        ],
        "responses": { "200": { "description": "Decision approval/rejection" } }
      }
    },
    "/self/health": {
      "get": {
        "operationId": "selfHealth",
        "summary": "SelfModel overall health score",
        "security": [],
        "responses": { "200": { "description": "{ healthy, overall }" } }
      }
    },
    "/self/capabilities": {
      "get": {
        "operationId": "listSelfCapabilities",
        "summary": "List registered SelfModel capabilities",
        "security": [],
        "responses": { "200": { "description": "Capability descriptors" } }
      },
      "post": {
        "operationId": "registerSelfCapability",
        "summary": "Register a capability at runtime",
        "requestBody": { "required": true, "content": { "application/json": {
          "schema": { "type": "object", "required": ["name"], "properties": {
            "name": { "type": "string" },
            "description": { "type": "string" },
            "required_cpu_percent": { "type": "number" },
            "required_memory_mb": { "type": "number" }
          } } } } },
        "responses": { "200": { "description": "Registration result" } }
      }
    },
    "/worldmodel/status": {
      "get": {
        "operationId": "worldModelStatus",
        "summary": "World model availability and transition/entity counts",
        "security": [],
        "responses": { "200": { "description": "Status JSON" } }
      }
    },
    "/worldmodel/observe": {
      "post": {
        "operationId": "worldModelObserve",
        "summary": "Observe a state transition (Dirichlet counts). Body: from|state, action, to|next_state",
        "security": [],
        "requestBody": { "required": true, "content": { "application/json": {
          "schema": { "type": "object", "required": ["action"], "properties": {
            "from": { "type": "string" },
            "state": { "type": "string", "description": "Alias for from" },
            "action": { "type": "string" },
            "to": { "type": "string" },
            "next_state": { "type": "string", "description": "Alias for to" }
          } } } } },
        "responses": { "200": { "description": "{ success, total_transitions }" } }
      }
    },
    "/worldmodel/entities": {
      "get": {
        "operationId": "listWorldModelEntities",
        "summary": "List Kalman-tracked entity IDs",
        "security": [],
        "responses": { "200": { "description": "{ entities, total }" } }
      }
    },
    "/worldmodel/entity": {
      "post": {
        "operationId": "registerWorldModelEntity",
        "summary": "Register entity with initial Kalman state",
        "security": [],
        "requestBody": { "required": true, "content": { "application/json": {
          "schema": { "type": "object", "required": ["id"], "properties": {
            "id": { "type": "string" },
            "dimensions": { "type": "integer", "default": 3 },
            "initial_values": { "type": "array", "items": { "type": "number" } }
          } } } } },
        "responses": { "200": { "description": "Registration result" } }
      }
    },
    "/worldmodel/causal": {
      "get": {
        "operationId": "getCausalGraph",
        "summary": "Dump causal graph edges",
        "security": [],
        "responses": { "200": { "description": "Causal graph" } }
      }
    },
    "/worldmodel/causal/edge": {
      "post": {
        "operationId": "addCausalEdge",
        "summary": "Add causal edge",
        "security": [],
        "responses": { "200": { "description": "Edge result" } }
      }
    },
    "/worldmodel/causal/intervention": {
      "post": {
        "operationId": "causalIntervention",
        "summary": "do-calculus intervention query",
        "security": [],
        "responses": { "200": { "description": "Intervention result" } }
      }
    },
    "/worldmodel/causal/counterfactual": {
      "post": {
        "operationId": "causalCounterfactual",
        "summary": "Structural counterfactual query",
        "security": [],
        "responses": { "200": { "description": "Counterfactual result" } }
      }
    },
    "/memory/live_beliefs": {
      "get": {
        "operationId": "getLiveBeliefs",
        "summary": "Unified live beliefs (symbolic + hypotheses + world + self/coherence)",
        "security": [],
        "parameters": [
          { "name": "actor", "in": "query", "schema": { "type": "string" } },
          { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 5 } }
        ],
        "responses": { "200": { "description": "Merged beliefs surface" } }
      }
    },
    "/memory/link": {
      "post": {
        "operationId": "linkMemories",
        "summary": "Create directed graph edge between two memory records",
        "security": [],
        "requestBody": { "required": true, "content": { "application/json": {
          "schema": { "type": "object", "properties": {
            "source_id": { "type": "string", "format": "uuid" },
            "target_id": { "type": "string", "format": "uuid" },
            "relation": { "type": "string", "default": "related" }
          } } } } },
        "responses": { "200": { "description": "Link result" } }
      }
    },
    "/memory/reflect": {
      "post": {
        "operationId": "memoryReflect",
        "summary": "CoT / hypothesis sampling via AureusBridge",
        "security": [],
        "responses": { "200": { "description": "Hypotheses" } }
      }
    },
    "/memory/hypotheses": {
      "get": {
        "operationId": "listHypotheses",
        "summary": "List current hypotheses",
        "security": [],
        "responses": { "200": { "description": "Hypothesis list" } }
      }
    },
    "/topo/ppr": {
      "get": {
        "operationId": "topoPpr",
        "summary": "Personalized PageRank over CausalTopoGraph",
        "parameters": [
          { "name": "seed", "in": "query", "schema": { "type": "string" } },
          { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 10 } }
        ],
        "responses": { "200": { "description": "{ results: [{id, score}] }" } }
      }
    },
    "/topo/check-edge": {
      "post": {
        "operationId": "topoCheckEdge",
        "summary": "Would causal edge from→to contradict topo?",
        "responses": { "200": { "description": "{ would_contradict, report }" } }
      }
    },
    "/topo/deconstruct": {
      "post": {
        "operationId": "topoDeconstruct",
        "summary": "Parse hypothesis text into nodes/edges (rules + optional llm_json)",
        "responses": { "200": { "description": "DeconstructedHypothesis" } }
      }
    },
    "/topo/apply-hyp": {
      "post": {
        "operationId": "topoApplyHyp",
        "summary": "Deconstruct text and apply causal edges to live topo graph",
        "responses": { "200": { "description": "Apply result" } }
      }
    }
  }
}"##;
