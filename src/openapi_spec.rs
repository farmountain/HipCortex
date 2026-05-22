//! OpenAPI 3.0 specification — served at GET /openapi.json
//! Update this when adding new endpoints.

pub const OPENAPI_SPEC: &str = r##"{
  "openapi": "3.0.3",
  "info": {
    "title": "HipCortex Memory Engine",
    "description": "Persistent causal memory for AI agents. Sub-millisecond writes, temporal decay, causal world model, GDPR-compliant.",
    "version": "0.2.0",
    "license": { "name": "Apache 2.0", "url": "https://opensource.org/licenses/Apache-2.0" },
    "contact": { "url": "https://github.com/farmountain/HipCortex" }
  },
  "servers": [
    { "url": "https://hipcortex.fly.dev", "description": "Managed SaaS (Frankfurt)" },
    { "url": "http://localhost:3030", "description": "Local self-hosted" }
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
      "responses": { "200": { "description": "Latest records" } } } }
  }
}"##;
