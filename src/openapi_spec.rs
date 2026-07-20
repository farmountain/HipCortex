//! OpenAPI 3.0 specification — served at GET /openapi.json
//! Update this when adding new endpoints.

pub const OPENAPI_SPEC: &str = r##"{
  "openapi": "3.0.3",
  "info": {
    "title": "HipCortex Memory Engine",
    "description": "Persistent causal memory for AI agents. Sub-millisecond writes, temporal decay, causal world model, GDPR-compliant.",
    "version": "0.5.0",
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
