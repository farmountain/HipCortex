## Why

External consumers (Claude agents, documentation, onboarding materials) refer to memory tiers using cognitive-science terminology: `Episodic`, `Semantic`, `LongTerm`, `ShortTerm`, `Working`, `Reflexive`, `Perceptual`. The Rust server stores records using its own enum names: `Temporal`, `Symbolic`, `Procedural`, `Reflexion`, `Perception`. This mismatch causes confusion and 400 errors when agents pass cognitive-science names to the API. The recent type-tightening in `sdd-post-session-hardening` locked the union to Rust names — correct for the server's internal contract, but unresolved for external consumers.

## What Changes

- **Server-side alias normalization in `handle_add_memory`**: Before deserializing `record_type`, normalize cognitive-science aliases to their canonical Rust enum equivalents. The mapping is input-only — stored and returned values remain canonical.
- **Decision: `LongTerm → Symbolic`**: Long-term facts are semantic knowledge stored in the symbolic store. This is the canonical mapping.
- **Update MCP Python server tool schema**: Add `Episodic`, `Semantic`, `LongTerm` to the `record_type` enum in the `add_memory` tool definition.
- **Update TypeScript SDK types**: Document aliases in JSDoc, optionally expand the union type to include them.

## Capabilities

### New Capabilities

- `record-type-aliases`: Server-side normalization layer accepting cognitive-science names as valid `record_type` inputs. Full alias table: `Episodic→Temporal`, `ShortTerm→Temporal`, `Working→Temporal`, `Semantic→Symbolic`, `LongTerm→Symbolic`, `Reflexive→Reflexion`, `Perceptual→Perception`.

### Modified Capabilities

- `memory-ingestion`: `POST /memory/add` now accepts a broader set of `record_type` values without returning a deserialization error.

## Impact

- **Files**: `src/web_server.rs` (normalization in `handle_add_memory` or via custom Deserialize on `RecordType`), `sdk/mcp/server.py` (tool schema update), `sdk/typescript/src/types.ts` (JSDoc + optional union expansion).
- **No breaking changes**: all existing canonical names continue to work identically.
- **Build**: `cargo check` + `npm run build` (TypeScript SDK).
