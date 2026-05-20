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
            "description": "Auto-expire after N seconds" }
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
    "/health": { "get": { "summary": "Health check", "security": [],
      "responses": { "200": { "description": "ok" } } } },
    "/stats": { "get": { "summary": "Live server statistics (public)", "security": [],
      "responses": { "200": { "description": "Stats JSON" } } } },
    "/pricing": { "get": { "summary": "Pricing page HTML", "security": [],
      "responses": { "200": { "description": "HTML" } } } },
    "/openapi.json": { "get": { "summary": "This OpenAPI spec", "security": [],
      "responses": { "200": { "description": "OpenAPI 3.0 JSON" } } } },
    "/memory/add": { "post": { "summary": "Store a memory record",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/AddMemoryRequest" } } } },
      "responses": { "200": { "description": "Success" } } } },
    "/memory/bulk": { "post": { "summary": "Store multiple records",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/BulkAddRequest" } } } },
      "responses": { "200": { "description": "Bulk result" } } } },
    "/memory/query": { "get": { "summary": "Query memory records",
      "parameters": [
        { "name": "actor", "in": "query", "schema": { "type": "string" } },
        { "name": "action", "in": "query", "schema": { "type": "string" } },
        { "name": "record_type", "in": "query", "schema": { "type": "string" } },
        { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 100 } }
      ],
      "responses": { "200": { "description": "Records" } } } },
    "/memory/search": { "post": { "summary": "Semantic + keyword search",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/SearchRequest" } } } },
      "responses": { "200": { "description": "Search results" } } } },
    "/memory/embed": { "post": { "summary": "Auto-embed then store",
      "requestBody": { "required": true, "content": { "application/json": {
        "schema": { "$ref": "#/components/schemas/EmbedAndAddRequest" } } } },
      "responses": { "200": { "description": "Stored with embedding" } } } },
    "/memory/forget/{actor}": { "delete": { "summary": "GDPR right-to-forget",
      "parameters": [{ "name": "actor", "in": "path", "required": true,
        "schema": { "type": "string" } }],
      "responses": { "200": { "description": "Deleted" } } } },
    "/coherence/status": { "get": { "summary": "Coherence metrics",
      "responses": { "200": { "description": "Coherence state" } } } },
    "/tier": { "get": { "summary": "API key tier info",
      "responses": { "200": { "description": "Tier + limits" } } } },
    "/graph": { "get": { "summary": "Full symbolic graph",
      "security": [],
      "responses": { "200": { "description": "Nodes and edges" } } } }
  }
}"##;
