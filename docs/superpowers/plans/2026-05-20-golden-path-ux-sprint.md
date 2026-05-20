# HipCortex Golden Path UX Sprint — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate every friction point in the first-use experience so the path from "landed on GitHub" to "working memory integration" takes under 5 minutes.

**Architecture:** 5 independent feature additions: (1) npm publish so `npm install hipcortex` works, (2) async LangChain memory wrapper for async Python stacks, (3) unified search-with-embedding endpoint, (4) graceful SIGINT shutdown + systemd service template, (5) MCP server exposing HipCortex tools to Cursor/Claude Code/Windsurf.

**Tech Stack:** Rust/Axum 0.6 (Tasks 3+4), Python/asyncio (Task 2), Python stdlib + requests (Task 5), npm + TypeScript (Task 1)

---

## V(state) — Golden Path Value Function

```python
def V_golden_path(S):
    """
    Maximize frictionless first-use → star conversion.
    Gate: each gap blocks a distinct user segment from reaching 'aha moment'.
    """
    # Segment unlocks (binary gates — friction either exists or doesn't)
    npm_gate     = float(S.npm_published)         # JS/TS devs blocked until this
    async_gate   = float(S.async_langchain)       # async Python stacks blocked
    search_gate  = float(S.unified_search_embed)  # local AI users need 2 calls today
    shutdown_gate = float(S.graceful_shutdown)    # Pi/edge users lose data on Ctrl+C
    mcp_gate     = float(S.mcp_server_shipped)   # Cursor/Claude Code/Windsurf blocked

    # Each gate = multiplicative unlock for that segment's conversion rate
    star_rate = (
        S.github_stars_baseline * 1.0 +       # baseline
        S.github_stars_baseline * 0.4 * npm_gate +    # +40% from JS community
        S.github_stars_baseline * 0.3 * async_gate +  # +30% from async Python
        S.github_stars_baseline * 0.2 * search_gate + # +20% from local AI
        S.github_stars_baseline * 0.1 * shutdown_gate + # +10% from edge/Pi
        S.github_stars_baseline * 8.0 * mcp_gate      # +8× from coding agent wave
    )

    paying_conversion = 0.02  # 2% of star-ers convert to Pro
    ARR_per_team = 1200       # avg $100/mo

    return star_rate * paying_conversion * ARR_per_team

# Current: S.npm_published=0, S.async_langchain=0, S.unified_search_embed=0,
#          S.graceful_shutdown=0, S.mcp_server_shipped=0
# → V = stars_baseline * 0.02 * 1200 ≈ $0 ARR (zero stars today)
#
# After all 5 tasks:
# → V = stars_baseline * 10.0 * 0.02 * 1200 ≈ 20× ARR multiplier
```

---

## File Map

| File | Action | Task |
|------|--------|------|
| `sdk/typescript/package.json` | MODIFY — add publishConfig | Task 1 |
| `.github/workflows/publish-npm.yml` | CREATE — npm publish on release | Task 1 |
| `sdk/python/hipcortex/langchain_memory.py` | MODIFY — add AsyncHipCortexMemory | Task 2 |
| `sdk/python/hipcortex/__init__.py` | MODIFY — export AsyncHipCortexMemory | Task 2 |
| `sdk/python/tests/test_async_langchain.py` | CREATE — async memory tests | Task 2 |
| `src/web_server.rs` | MODIFY — add embedding_model to SearchMemoryRequest | Task 3 |
| `src/bin/webserver.rs` | MODIFY — SIGINT handler + flush | Task 4a |
| `docs/systemd/hipcortex.service` | CREATE — systemd unit file | Task 4b |
| `sdk/mcp/server.py` | CREATE — MCP JSON-RPC stdio server | Task 5 |
| `sdk/mcp/README.md` | CREATE — Cursor/Claude Code install guide | Task 5 |
| `sdk/mcp/install.sh` | CREATE — one-line install script | Task 5 |

---

## Phase 2 Backlog (do NOT implement now — gate on 500 stars)

- GitHub Copilot Extension SDK adapter
- `hipcortex-cli` terminal tool (Aider/SWE-agent integration)
- Continue.dev context provider plugin
- Pro tier Stripe billing flow
- Multi-tenancy / namespace isolation
- Webhooks on memory events
- Kubernetes Helm chart

---

## Task 1: npm publish — make `npm install hipcortex` work

**Files:**
- Modify: `sdk/typescript/package.json`
- Create: `.github/workflows/publish-npm.yml`

- [ ] **Step 1: Add publishConfig to package.json**

Read `sdk/typescript/package.json` first. Then edit it to add `publishConfig` and `repository` fields. The full updated package.json:

```json
{
  "name": "hipcortex",
  "version": "0.2.0",
  "description": "Persistent causal memory for AI agents — LangChain, LlamaIndex, AutoGen, Vercel AI SDK",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist", "README.md"],
  "publishConfig": {
    "access": "public",
    "registry": "https://registry.npmjs.org/"
  },
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "prepublishOnly": "npm run build"
  },
  "keywords": ["ai", "memory", "langchain", "autogen", "llm", "agents", "rag", "hipcortex"],
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/farmountain/HipCortex.git",
    "directory": "sdk/typescript"
  },
  "homepage": "https://github.com/farmountain/HipCortex#readme",
  "bugs": { "url": "https://github.com/farmountain/HipCortex/issues" },
  "devDependencies": {
    "@types/jest": "^29.0.0",
    "@types/node": "^20.0.0",
    "jest": "^29.0.0",
    "ts-jest": "^29.0.0",
    "typescript": "^5.0.0"
  },
  "jest": {
    "preset": "ts-jest",
    "testEnvironment": "node"
  }
}
```

- [ ] **Step 2: Build the TypeScript package (generates dist/)**

```bash
cd sdk/typescript
npm install
npm run build
```
Expected: `dist/index.js`, `dist/index.d.ts`, `dist/client.js`, `dist/types.js` created.

- [ ] **Step 3: Dry-run publish to verify package contents**

```bash
cd sdk/typescript
npm pack --dry-run
```
Expected output includes:
```
npm notice Files included in hipcortex-0.2.0.tgz:
npm notice   dist/index.js
npm notice   dist/index.d.ts
npm notice   dist/client.js
npm notice   dist/types.js
npm notice   README.md
```
If `dist/` files are missing, `npm run build` failed — check TypeScript errors.

- [ ] **Step 4: Create `.github/workflows/publish-npm.yml`**

```yaml
name: Publish npm package

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      dry_run:
        description: "Dry run (no actual publish)"
        type: boolean
        default: false

jobs:
  publish:
    name: Build + publish hipcortex to npm
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: sdk/typescript

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Install dependencies
        run: npm ci

      - name: Run tests
        run: npm test

      - name: Build
        run: npm run build

      - name: Publish to npm
        if: ${{ !inputs.dry_run }}
        run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Dry run (check contents only)
        if: ${{ inputs.dry_run }}
        run: npm pack --dry-run
```

- [ ] **Step 5: Manual publish (do this NOW — don't wait for CI)**

```bash
# One-time: login to npm (opens browser)
cd sdk/typescript
npm login

# Build + publish
npm run build
npm publish --access public
```
Expected: `+ hipcortex@0.2.0` success message.

Verify at: https://www.npmjs.com/package/hipcortex

- [ ] **Step 6: Add NPM_TOKEN secret to GitHub**

Go to: https://www.npmjs.com/settings/YOUR_USERNAME/tokens
Create "Automation" token. Copy it.
Go to: https://github.com/farmountain/HipCortex/settings/secrets/actions
Add secret: `NPM_TOKEN` = the token.

- [ ] **Step 7: Commit**

```bash
git add sdk/typescript/package.json .github/workflows/publish-npm.yml sdk/typescript/dist/
git commit -m "feat: npm publish — hipcortex on npm registry

npm install hipcortex now works.
publishConfig set to public access.
CI publishes on every GitHub release."
```

- [ ] **Step 8: Verify installation works**

In a fresh terminal with no local npm link:
```bash
mkdir /tmp/test-npm && cd /tmp/test-npm
npm init -y
npm install hipcortex
node -e "const { HipCortexClient } = require('hipcortex'); console.log('✓ import works');"
```
Expected: `✓ import works`

---

## Task 2: AsyncHipCortexMemory — async LangChain memory

**Files:**
- Modify: `sdk/python/hipcortex/langchain_memory.py`
- Modify: `sdk/python/hipcortex/__init__.py`
- Create: `sdk/python/tests/test_async_langchain.py`

- [ ] **Step 1: Write the failing tests first**

Create `sdk/python/tests/test_async_langchain.py`:

```python
"""Tests for AsyncHipCortexMemory — run: pytest sdk/python/tests/test_async_langchain.py -v"""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch


@pytest.mark.asyncio
async def test_async_load_memory_variables_empty():
    """Returns empty history string when no records exist."""
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient

    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.get_conversation_history = AsyncMock(return_value=[])

    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-1")
    result = await memory.aload_memory_variables({})

    assert result == {"history": ""}
    mock_client.get_conversation_history.assert_awaited_once_with("sess-1", limit=50)


@pytest.mark.asyncio
async def test_async_load_memory_variables_with_records():
    """Formats human/AI messages in history string."""
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient

    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.get_conversation_history = AsyncMock(return_value=[
        {"action": "human_message", "target": "Hello", "timestamp": "2026-01-01T00:00:00Z"},
        {"action": "ai_message",    "target": "Hi!",   "timestamp": "2026-01-01T00:00:01Z"},
    ])

    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-2")
    result = await memory.aload_memory_variables({})

    assert result["history"] == "Human: Hello\nAI: Hi!"


@pytest.mark.asyncio
async def test_async_save_context():
    """Saves human + AI messages asynchronously."""
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient

    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.add_human_message = AsyncMock(return_value={"success": True})
    mock_client.add_ai_message    = AsyncMock(return_value={"success": True})

    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-3")
    await memory.asave_context({"input": "How are you?"}, {"output": "I am fine."})

    mock_client.add_human_message.assert_awaited_once_with("sess-3", "How are you?")
    mock_client.add_ai_message.assert_awaited_once_with("sess-3", "I am fine.")


@pytest.mark.asyncio
async def test_async_clear():
    """Clear calls forget on the client."""
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient

    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.forget = AsyncMock(return_value={"success": True})

    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-4")
    await memory.aclear()

    mock_client.forget.assert_awaited_once_with("sess-4")


def test_memory_variables_property():
    """memory_variables returns the configured key."""
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient

    mock_client = MagicMock(spec=AsyncHipCortexClient)
    memory = AsyncHipCortexMemory(client=mock_client, session_id="x", memory_key="chat_history")
    assert memory.memory_variables == ["chat_history"]
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd sdk/python
pytest tests/test_async_langchain.py -v 2>&1 | head -15
```
Expected: `ImportError: cannot import name 'AsyncHipCortexMemory'`

- [ ] **Step 3: Add AsyncHipCortexMemory to langchain_memory.py**

Read `sdk/python/hipcortex/langchain_memory.py` first. Then **append** this class at the bottom of the file (after `HipCortexMemory`), without touching existing code:

```python

class AsyncHipCortexMemory:
    """Async-native LangChain ``BaseMemory`` backed by AsyncHipCortexClient.

    Use with async LangChain chains (LangChain 0.2+, FastAPI, Django async).
    Implements ``aload_memory_variables`` and ``asave_context`` as coroutines.

    Sync ``load_memory_variables`` / ``save_context`` are also provided as
    thin wrappers using ``asyncio.get_event_loop().run_until_complete()`` for
    compatibility with sync callers, but prefer the async variants.

    Usage::

        from hipcortex import AsyncHipCortexClient
        from hipcortex.langchain_memory import AsyncHipCortexMemory

        async def chat(user_input: str, session_id: str):
            client = AsyncHipCortexClient("http://localhost:3030")
            memory = AsyncHipCortexMemory(client=client, session_id=session_id)
            history = await memory.aload_memory_variables({})
            # ... call LLM with history["history"] ...
            await memory.asave_context({"input": user_input}, {"output": ai_response})
    """

    def __init__(
        self,
        client: "Any",  # AsyncHipCortexClient — quoted to avoid circular import
        session_id: str = "default",
        memory_key: str = "history",
        human_prefix: str = "Human",
        ai_prefix: str = "AI",
        max_limit: int = 50,
    ) -> None:
        self._client = client
        self.session_id = session_id
        self.memory_key = memory_key
        self.human_prefix = human_prefix
        self.ai_prefix = ai_prefix
        self.max_limit = max_limit

    # ------------------------------------------------------------------
    # Required by BaseMemory interface (used as property)
    # ------------------------------------------------------------------

    @property
    def memory_variables(self) -> List[str]:
        return [self.memory_key]

    # ------------------------------------------------------------------
    # Async-native methods (preferred)
    # ------------------------------------------------------------------

    async def aload_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Fetch conversation history and return as formatted string."""
        records = await self._client.get_conversation_history(
            self.session_id, limit=self.max_limit
        )
        records.sort(key=lambda r: r.get("timestamp", ""))
        lines: List[str] = []
        for rec in records:
            action = rec.get("action", "")
            target = rec.get("target", "")
            if action == "human_message":
                lines.append(f"{self.human_prefix}: {target}")
            elif action == "ai_message":
                lines.append(f"{self.ai_prefix}: {target}")
        return {self.memory_key: "\n".join(lines)}

    async def asave_context(
        self, inputs: Dict[str, Any], outputs: Dict[str, Any]
    ) -> None:
        """Persist a human→AI exchange asynchronously."""
        human_text = inputs.get("input") or inputs.get("human_input") or ""
        ai_text    = outputs.get("output") or outputs.get("response") or ""
        if human_text:
            await self._client.add_human_message(self.session_id, str(human_text))
        if ai_text:
            await self._client.add_ai_message(self.session_id, str(ai_text))

    async def aclear(self) -> None:
        """GDPR forget for this session — delete all memories."""
        await self._client.forget(self.session_id)

    # ------------------------------------------------------------------
    # Sync wrappers (for compatibility with sync callers)
    # ------------------------------------------------------------------

    def load_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Sync fallback — runs the async method in the current event loop."""
        import asyncio
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                # In Jupyter or async context — create task in running loop
                import concurrent.futures
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    future = pool.submit(asyncio.run, self.aload_memory_variables(inputs))
                    return future.result()
            return loop.run_until_complete(self.aload_memory_variables(inputs))
        except RuntimeError:
            return asyncio.run(self.aload_memory_variables(inputs))

    def save_context(self, inputs: Dict[str, Any], outputs: Dict[str, Any]) -> None:
        """Sync fallback — runs the async method in the current event loop."""
        import asyncio
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import concurrent.futures
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    pool.submit(asyncio.run, self.asave_context(inputs, outputs)).result()
                return
            loop.run_until_complete(self.asave_context(inputs, outputs))
        except RuntimeError:
            asyncio.run(self.asave_context(inputs, outputs))

    def clear(self) -> None:
        """Sync fallback — runs the async clear method."""
        import asyncio
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                import concurrent.futures
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    pool.submit(asyncio.run, self.aclear()).result()
                return
            loop.run_until_complete(self.aclear())
        except RuntimeError:
            asyncio.run(self.aclear())
```

- [ ] **Step 4: Export from `__init__.py`**

Read `sdk/python/hipcortex/__init__.py`. Add `AsyncHipCortexMemory` to the imports and `__all__`:

```python
"""HipCortex Python SDK — AI memory engine client."""

from .client import HipCortexClient
from .async_client import AsyncHipCortexClient
from .langchain_memory import HipCortexMemory, AsyncHipCortexMemory
from .llamaindex_storage import HipCortexStorageContext

__version__ = "0.2.0"
__all__ = [
    "HipCortexClient",
    "AsyncHipCortexClient",
    "HipCortexMemory",
    "AsyncHipCortexMemory",
    "HipCortexStorageContext",
]
```

- [ ] **Step 5: Run tests — all must pass**

```bash
cd sdk/python
pytest tests/test_async_langchain.py -v
```
Expected:
```
tests/test_async_langchain.py::test_async_load_memory_variables_empty PASSED
tests/test_async_langchain.py::test_async_load_memory_variables_with_records PASSED
tests/test_async_langchain.py::test_async_save_context PASSED
tests/test_async_langchain.py::test_async_clear PASSED
tests/test_async_langchain.py::test_memory_variables_property PASSED
5 passed
```

- [ ] **Step 6: Commit**

```bash
git add sdk/python/hipcortex/langchain_memory.py \
        sdk/python/hipcortex/__init__.py \
        sdk/python/tests/test_async_langchain.py
git commit -m "feat: AsyncHipCortexMemory — async LangChain BaseMemory

Wraps AsyncHipCortexClient. aload_memory_variables() and asave_context()
are true coroutines. Sync load_memory_variables()/save_context() wrappers
provided for compatibility. Compatible with LangChain 0.2+ async chains,
FastAPI, Django async, LangGraph.

from hipcortex import AsyncHipCortexMemory"
```

---

## Task 3: POST /memory/search + embedding_model (unified search)

**Files:**
- Modify: `src/web_server.rs` (add `embedding_model` to `SearchMemoryRequest` + handler logic)

- [ ] **Step 1: Read src/web_server.rs around SearchMemoryRequest**

Search for `SearchMemoryRequest` and `handle_search_memory` to understand current structure. The struct currently has `query`, `embedding`, `limit`. The handler calls `ms.search_semantic(req.embedding.as_deref(), &req.query, limit)`.

- [ ] **Step 2: Add embedding_model field to SearchMemoryRequest**

Find:
```rust
pub struct SearchMemoryRequest {
    /// Free-text query (used for keyword matching, and as label when no embedding)
    pub query: String,
    /// Optional embedding vector for cosine similarity search
    pub embedding: Option<Vec<f64>>,
    /// Max results (default 10)
    pub limit: Option<usize>,
}
```

Replace with:
```rust
pub struct SearchMemoryRequest {
    /// Free-text query — used for keyword matching and as input to embedding_model
    pub query: String,
    /// Pre-computed embedding vector for cosine similarity (caller provides)
    pub embedding: Option<Vec<f64>>,
    /// Max results (default 10)
    pub limit: Option<usize>,
    /// If provided, auto-generate query embedding before search.
    /// Format: "ollama/<model>" or "openai/<model>"
    /// Example: "ollama/nomic-embed-text"
    pub embedding_model: Option<String>,
}
```

- [ ] **Step 3: Update handle_search_memory to call embedding API when embedding_model is set**

Find `handle_search_memory`. The function body currently starts with:
```rust
let limit = req.limit.unwrap_or(10).min(100);
match store.lock() {
```

Replace the entire function body with this implementation that auto-generates the query embedding when `embedding_model` is set:

```rust
#[cfg(feature = "web-server")]
async fn handle_search_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<SearchMemoryRequest>,
) -> Result<Json<SearchMemoryResponse>, (StatusCode, Json<SearchMemoryResponse>)> {
    let limit = req.limit.unwrap_or(10).min(100);

    // Auto-generate query embedding when embedding_model is specified
    // and no pre-computed embedding was provided
    let resolved_embedding: Option<Vec<f64>> = if req.embedding.is_some() {
        req.embedding.clone()
    } else if let Some(model_str) = &req.embedding_model {
        // Reuse the same embedding generation logic as handle_embed_and_add
        let embedding = generate_embedding(model_str, &req.query).await;
        match embedding {
            Ok(v) if !v.is_empty() => Some(v),
            Ok(_) => None, // empty vector = fall back to keyword search
            Err(e) => return Err((
                StatusCode::BAD_GATEWAY,
                Json(SearchMemoryResponse { results: vec![], total: 0 }),
            )),
        }
    } else {
        None
    };

    let now_ts = chrono::Utc::now().timestamp();
    match store.lock() {
        Ok(ms) => {
            let results = ms.search_semantic(
                resolved_embedding.as_deref(),
                &req.query,
                limit,
            );
            let response_results = results
                .into_iter()
                .filter(|(r, _)| r.expires_at.map_or(true, |exp| exp > now_ts))
                .map(|(r, score)| SearchResult {
                    score,
                    record: MemoryRecordResponse {
                        id:          r.id.to_string(),
                        record_type: format!("{:?}", r.record_type),
                        timestamp:   r.timestamp.to_rfc3339(),
                        actor:       r.actor.clone(),
                        action:      r.action.clone(),
                        target:      r.target.clone(),
                        metadata:    r.metadata.clone(),
                        integrity:   r.integrity.clone(),
                    },
                })
                .collect::<Vec<_>>();
            let total = response_results.len();
            Ok(Json(SearchMemoryResponse { results: response_results, total }))
        }
        Err(_e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SearchMemoryResponse { results: vec![], total: 0 }),
        )),
    }
}
```

- [ ] **Step 4: Extract generate_embedding helper function**

The embedding generation logic is duplicated between `handle_embed_and_add` and the new search handler. Extract it as a shared async fn. Add this function BEFORE `handle_search_memory`:

```rust
/// Generate an embedding vector by calling Ollama or OpenAI.
/// model_str format: "ollama/<model>" or "openai/<model>"
#[cfg(feature = "web-server")]
async fn generate_embedding(model_str: &str, text: &str) -> Result<Vec<f64>, String> {
    if model_str.starts_with("ollama/") {
        let model = &model_str["ollama/".len()..];
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let body = serde_json::json!({ "model": model, "prompt": text });
        let resp = client
            .post(format!("{}/api/embeddings", ollama_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default())
    } else if model_str.starts_with("openai/") {
        let model = &model_str["openai/".len()..];
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let body = serde_json::json!({ "model": model, "input": text });
        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["data"][0]["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default())
    } else {
        Err(format!(
            "embedding_model must be 'ollama/<model>' or 'openai/<model>', got: {}",
            model_str
        ))
    }
}
```

Also update `handle_embed_and_add` to use `generate_embedding` instead of its own inline logic. Find the existing inline logic in that handler and replace it with:
```rust
let embedding = generate_embedding(&req.embedding_model, &req.target)
    .await
    .map_err(|e| (
        StatusCode::BAD_GATEWAY,
        Json(AddMemoryResponse { success: false, record_id: None, error: Some(e) }),
    ))?;
```

- [ ] **Step 5: Build verify**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error|Finished"
```
Expected: `Finished dev profile`

- [ ] **Step 6: Commit**

```bash
git add src/web_server.rs
git commit -m "feat: POST /memory/search + embedding_model — unified search

Single call: POST /memory/search {query: '...', embedding_model: 'ollama/nomic-embed-text'}
Server auto-generates query embedding, returns cosine-ranked results.
Previously required 2 calls (POST /memory/embed then POST /memory/search).
Extracted generate_embedding() helper used by both /memory/embed and /memory/search."
```

---

## Task 4: Graceful SIGINT shutdown + systemd template

### Task 4a: Graceful shutdown in webserver binary

**Files:**
- Modify: `src/bin/webserver.rs`

- [ ] **Step 1: Read current src/bin/webserver.rs**

The file currently ends with:
```rust
web_server::run_with_memory(addr, memory_store).await;
Ok(())
```

- [ ] **Step 2: Add graceful shutdown with SIGINT handler**

Replace the final lines of `main()`:
```rust
web_server::run_with_memory(addr, memory_store).await;
Ok(())
```
With:
```rust
// Graceful shutdown: catch Ctrl+C / SIGTERM, flush MemoryStore before exit
// This prevents partial writes corrupting memory.jsonl on Raspberry Pi / edge devices
let store_for_signal = memory_store.clone();
tokio::select! {
    _ = web_server::run_with_memory(addr, memory_store) => {
        println!("Server exited normally.");
    }
    _ = tokio::signal::ctrl_c() => {
        println!("\nShutdown signal received — flushing memory store...");
        if let Ok(mut ms) = store_for_signal.lock() {
            match ms.flush() {
                Ok(_)  => println!("Flush complete. Data is safe. Goodbye."),
                Err(e) => eprintln!("Flush error: {}. Manual recovery may be needed.", e),
            }
        }
    }
}
Ok(())
```

Also add `tokio::signal` to the imports at the top of the file. Check if it needs a feature flag — `tokio::signal` requires `features = ["signal"]` in Cargo.toml. Find `tokio` in Cargo.toml dependencies and add `"signal"` to its features list. If `tokio` already has `features = ["full"]`, no change needed.

- [ ] **Step 3: Build verify**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error|Finished"
```
Expected: `Finished dev profile`

### Task 4b: systemd service template

**Files:**
- Create: `docs/systemd/hipcortex.service`
- Create: `docs/systemd/README.md`

- [ ] **Step 4: Create systemd service file**

Create `docs/systemd/hipcortex.service`:

```ini
[Unit]
Description=HipCortex AI Memory Engine
Documentation=https://github.com/farmountain/HipCortex
After=network.target
Wants=network.target

[Service]
Type=simple
User=hipcortex
Group=hipcortex

# Binary location — adjust to where you placed the hipcortex binary
ExecStart=/usr/local/bin/hipcortex

# Data directory — all memory records and audit logs go here
Environment=DATA_DIR=/var/lib/hipcortex
Environment=PORT=3030
Environment=RUST_LOG=info

# API key tiers (optional) — format: key1:free,key2:pro,key3:team
# EnvironmentFile=/etc/hipcortex/keys.env

# Restart policy
Restart=on-failure
RestartSec=5s
StartLimitBurst=5
StartLimitIntervalSec=60

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/hipcortex
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 5: Create systemd README**

Create `docs/systemd/README.md`:

```markdown
# HipCortex systemd Service

Run HipCortex as a systemd service on Linux (Ubuntu, Debian, Raspberry Pi OS, etc.).

## Quick install

```bash
# 1. Download binary
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-arm64 \
  -o /usr/local/bin/hipcortex && chmod +x /usr/local/bin/hipcortex
# (use hipcortex-linux-amd64 for x86_64)

# 2. Create user and data directory
sudo useradd -r -s /bin/false -m -d /var/lib/hipcortex hipcortex
sudo mkdir -p /var/lib/hipcortex
sudo chown hipcortex:hipcortex /var/lib/hipcortex

# 3. Install service file
sudo cp hipcortex.service /etc/systemd/system/

# 4. Enable and start
sudo systemctl daemon-reload
sudo systemctl enable hipcortex
sudo systemctl start hipcortex

# 5. Verify
sudo systemctl status hipcortex
curl http://localhost:3030/health  # → ok
```

## Configuration

Edit `/etc/systemd/system/hipcortex.service` then:
```bash
sudo systemctl daemon-reload && sudo systemctl restart hipcortex
```

For API keys, create `/etc/hipcortex/keys.env`:
```
HIPCORTEX_API_KEYS=sk-free-abc:free,sk-pro-xyz:pro
```
Uncomment the `EnvironmentFile` line in the service file.

## Logs
```bash
sudo journalctl -u hipcortex -f
```
```

- [ ] **Step 6: Commit both**

```bash
git add src/bin/webserver.rs docs/systemd/
git commit -m "feat: graceful SIGINT shutdown + systemd service template

Graceful shutdown: Ctrl+C flushes MemoryStore before exit.
Prevents partial JSONL writes on Raspberry Pi / edge device power-off.
systemd service template for Linux self-hosted deployments.
Install guide in docs/systemd/README.md."
```

---

## Task 5: MCP Server — Cursor, Claude Code, Windsurf integration

**Files:**
- Create: `sdk/mcp/server.py`
- Create: `sdk/mcp/README.md`
- Create: `sdk/mcp/install.sh`
- Create: `sdk/mcp/__init__.py` (empty, marks as package)

- [ ] **Step 1: Write the failing tests first**

Create `sdk/mcp/test_server.py`:

```python
"""MCP server unit tests — run: pytest sdk/mcp/test_server.py -v"""
import json
import sys
import io
from unittest.mock import patch, MagicMock


def _call_server(input_lines: list[str]) -> list[dict]:
    """Feed lines to the server's stdin, collect JSON responses from stdout."""
    import importlib.util, importlib, types

    # Import server module without executing __main__
    spec = importlib.util.spec_from_file_location(
        "mcp_server",
        "sdk/mcp/server.py",
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    responses = []
    stdin_data = "\n".join(input_lines) + "\n"
    with patch("sys.stdin", io.StringIO(stdin_data)), \
         patch("sys.stdout", new_callable=io.StringIO) as mock_stdout:
        module.main()
        output = mock_stdout.getvalue()
    for line in output.strip().split("\n"):
        if line.strip():
            responses.append(json.loads(line))
    return responses


def test_initialize():
    """Server responds to initialize with capabilities."""
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}})
    ])
    assert len(resp) == 1
    assert resp[0]["id"] == 1
    assert resp[0]["result"]["capabilities"] == {"tools": {}}
    assert resp[0]["result"]["serverInfo"]["name"] == "hipcortex"


def test_tools_list():
    """tools/list returns all 4 tools."""
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    ])
    tools_resp = next(r for r in resp if r.get("id") == 2)
    tool_names = {t["name"] for t in tools_resp["result"]["tools"]}
    assert tool_names == {"add_memory", "search_memory", "forget_actor", "get_stats"}


def test_tools_call_add_memory():
    """tools/call add_memory calls POST /memory/add."""
    mock_resp = MagicMock()
    mock_resp.json.return_value = {"success": True, "record_id": "abc-123"}
    mock_resp.raise_for_status = MagicMock()

    with patch("requests.post", return_value=mock_resp):
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "add_memory",
                                   "arguments": {"actor": "project", "action": "decided",
                                                 "target": "Use JWT for auth"}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    content = call_resp["result"]["content"][0]["text"]
    assert "abc-123" in content


def test_tools_call_get_stats():
    """tools/call get_stats calls GET /stats."""
    mock_resp = MagicMock()
    mock_resp.json.return_value = {
        "total_records": 42, "unique_actors": 3,
        "by_type": {"Temporal": 40, "Reflexion": 2}
    }
    mock_resp.raise_for_status = MagicMock()

    with patch("requests.get", return_value=mock_resp):
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "get_stats", "arguments": {}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    content = call_resp["result"]["content"][0]["text"]
    assert "42" in content


def test_unknown_method_returns_error():
    """Unknown JSON-RPC methods return an error."""
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "nonexistent", "params": {}}),
    ])
    err_resp = next(r for r in resp if r.get("id") == 2)
    assert "error" in err_resp
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd sdk/mcp
pytest test_server.py -v 2>&1 | head -10
```
Expected: `FileNotFoundError` or `ModuleNotFoundError` — `server.py` doesn't exist yet.

- [ ] **Step 3: Create sdk/mcp/__init__.py (empty)**

```python
# HipCortex MCP server package
```

- [ ] **Step 4: Create sdk/mcp/server.py**

```python
#!/usr/bin/env python3
"""HipCortex MCP Server — Model Context Protocol server.

Exposes HipCortex memory as tools for Cursor, Claude Code, Windsurf, and
any other MCP-compatible AI coding assistant.

Protocol: JSON-RPC 2.0 over stdin/stdout (MCP spec 2024-11-05)
Dependencies: Python 3.9+ stdlib + requests

Usage:
    python server.py  # reads JSON-RPC from stdin, writes to stdout

Environment:
    HIPCORTEX_URL      HipCortex server URL (default: http://localhost:3030)
    HIPCORTEX_API_KEY  Optional X-Api-Key for managed SaaS tiers
"""

from __future__ import annotations

import json
import os
import sys
import urllib.parse
from typing import Any

import requests

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

HIPCORTEX_URL = os.getenv("HIPCORTEX_URL", "http://localhost:3030").rstrip("/")
API_KEY = os.getenv("HIPCORTEX_API_KEY", "")
TIMEOUT = int(os.getenv("HIPCORTEX_TIMEOUT", "10"))

# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _headers() -> dict:
    h = {"Content-Type": "application/json"}
    if API_KEY:
        h["X-Api-Key"] = API_KEY
    return h


def _get(path: str) -> dict:
    resp = requests.get(f"{HIPCORTEX_URL}{path}", headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()


def _post(path: str, body: dict) -> dict:
    resp = requests.post(f"{HIPCORTEX_URL}{path}", json=body, headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()


def _delete(path: str) -> dict:
    resp = requests.delete(f"{HIPCORTEX_URL}{path}", headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()

# ---------------------------------------------------------------------------
# Tool definitions (MCP schema)
# ---------------------------------------------------------------------------

TOOLS = [
    {
        "name": "add_memory",
        "description": (
            "Store a memory record in HipCortex. "
            "Use to remember decisions, code patterns, bug fixes, architectural choices, "
            "or any context that should persist across sessions."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["actor", "action", "target"],
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Scope identifier — use project name or 'global' (e.g. 'my-app', 'user-42')",
                },
                "action": {
                    "type": "string",
                    "description": "What happened (e.g. 'decided', 'implemented', 'fixed', 'noted', 'warned')",
                },
                "target": {
                    "type": "string",
                    "description": "The content to remember — be specific and self-contained",
                },
                "record_type": {
                    "type": "string",
                    "enum": ["Temporal", "Symbolic", "Procedural", "Reflexion", "Perception"],
                    "default": "Temporal",
                    "description": "Memory type: Temporal=time-sensitive, Symbolic=fact, Procedural=workflow, Reflexion=insight",
                },
                "ttl_seconds": {
                    "type": "integer",
                    "description": "Auto-expire after N seconds. Omit for permanent memory.",
                },
            },
        },
    },
    {
        "name": "search_memory",
        "description": (
            "Search stored memories by keyword. "
            "Use before starting a task to recall relevant past decisions, "
            "known issues, or architectural context."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to search for — use natural language",
                },
                "actor": {
                    "type": "string",
                    "description": "Filter by actor/scope (optional)",
                },
                "limit": {
                    "type": "integer",
                    "default": 10,
                    "description": "Max results to return",
                },
            },
        },
    },
    {
        "name": "forget_actor",
        "description": (
            "Delete all memories for a specific actor (GDPR right-to-forget). "
            "Use when starting fresh or cleaning up old project memories."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["actor"],
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Actor whose memories to delete",
                },
            },
        },
    },
    {
        "name": "get_stats",
        "description": "Get current memory store statistics: total records, types, unique actors.",
        "inputSchema": {
            "type": "object",
            "properties": {},
        },
    },
]

# ---------------------------------------------------------------------------
# Tool execution
# ---------------------------------------------------------------------------

def handle_add_memory(args: dict) -> str:
    body = {
        "actor": args["actor"],
        "action": args["action"],
        "target": args["target"],
    }
    if "record_type" in args:
        body["record_type"] = args["record_type"]
    if "ttl_seconds" in args:
        body["ttl_seconds"] = args["ttl_seconds"]
    result = _post("/memory/add", body)
    record_id = result.get("record_id", "unknown")
    return f"✓ Memory stored (id: {record_id})\n  [{args['action']}] {args['target']}"


def handle_search_memory(args: dict) -> str:
    limit = args.get("limit", 10)
    # Try semantic search first
    body: dict = {"query": args["query"], "limit": limit}
    result = _post("/memory/search", body)
    search_results = result.get("results", [])

    if search_results:
        lines = []
        for item in search_results:
            rec = item.get("record", {})
            score = item.get("score", 0.0)
            lines.append(
                f"• [{rec.get('action', '?')}] {rec.get('target', '')} "
                f"(actor: {rec.get('actor', '?')}, score: {score:.2f})"
            )
        return f"Found {len(lines)} result(s):\n" + "\n".join(lines)

    # Fallback: actor-filtered query
    params: dict = {"limit": limit}
    if "actor" in args:
        params["actor"] = args["actor"]
    qs = urllib.parse.urlencode(params)
    result2 = _get(f"/memory/query?{qs}")
    records = result2.get("records", [])
    if not records:
        return "No memories found."
    lines = [
        f"• [{r.get('action', '?')}] {r.get('target', '')} (actor: {r.get('actor', '?')})"
        for r in records[:limit]
    ]
    return f"Found {len(lines)} record(s):\n" + "\n".join(lines)


def handle_forget_actor(args: dict) -> str:
    actor = args["actor"]
    result = _delete(f"/memory/forget/{actor}")
    deleted = result.get("records_deleted", 0)
    symbolic = result.get("symbolic_nodes_deleted", 0)
    return f"✓ Deleted {deleted} memory records and {symbolic} symbolic nodes for '{actor}'."


def handle_get_stats(args: dict) -> str:
    result = _get("/stats")
    total = result.get("total_records", 0)
    actors = result.get("unique_actors", 0)
    by_type = result.get("by_type", {})
    metered = result.get("metering_enabled", False)
    lines = [
        f"HipCortex memory store:",
        f"  Total records:  {total}",
        f"  Unique actors:  {actors}",
        f"  Metering:       {'enabled' if metered else 'disabled (open mode)'}",
    ]
    if by_type:
        lines.append("  By type:")
        for t, count in sorted(by_type.items()):
            lines.append(f"    {t}: {count}")
    return "\n".join(lines)


def dispatch_tool(name: str, args: dict) -> str:
    handlers = {
        "add_memory":    handle_add_memory,
        "search_memory": handle_search_memory,
        "forget_actor":  handle_forget_actor,
        "get_stats":     handle_get_stats,
    }
    handler = handlers.get(name)
    if handler is None:
        raise ValueError(f"Unknown tool: {name}")
    return handler(args)

# ---------------------------------------------------------------------------
# JSON-RPC transport
# ---------------------------------------------------------------------------

def respond(id_: Any, result: Any = None, error: Any = None) -> None:
    msg: dict = {"jsonrpc": "2.0", "id": id_}
    if error is not None:
        msg["error"] = {"code": -32000, "message": str(error)}
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def main() -> None:
    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as e:
            respond(None, error=f"JSON parse error: {e}")
            continue

        method = req.get("method", "")
        id_ = req.get("id")           # None for notifications
        params = req.get("params", {})

        if method == "initialize":
            respond(id_, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "hipcortex", "version": "0.2.0"},
            })
        elif method == "initialized":
            pass  # notification — no response required
        elif method == "tools/list":
            respond(id_, {"tools": TOOLS})
        elif method == "tools/call":
            tool_name = params.get("name", "")
            tool_args = params.get("arguments", {})
            try:
                content = dispatch_tool(tool_name, tool_args)
                respond(id_, {"content": [{"type": "text", "text": content}]})
            except requests.RequestException as e:
                respond(id_, error=f"HipCortex server error: {e}")
            except Exception as e:
                respond(id_, error=str(e))
        elif method == "ping":
            respond(id_, {})
        else:
            if id_ is not None:  # notifications have no id, don't respond
                respond(id_, error=f"Unknown method: {method}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 5: Run tests — all must pass**

```bash
cd sdk/mcp
pip install requests pytest -q
pytest test_server.py -v
```
Expected:
```
test_server.py::test_initialize PASSED
test_server.py::test_tools_list PASSED
test_server.py::test_tools_call_add_memory PASSED
test_server.py::test_tools_call_get_stats PASSED
test_server.py::test_unknown_method_returns_error PASSED
5 passed
```

- [ ] **Step 6: Create install script sdk/mcp/install.sh**

```bash
#!/usr/bin/env bash
# HipCortex MCP server installer
# Usage: curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/install.sh | bash

set -e

INSTALL_DIR="${HOME}/.hipcortex-mcp"
REPO="https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/server.py"

echo "Installing HipCortex MCP server..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$REPO" -o "$INSTALL_DIR/server.py"
chmod +x "$INSTALL_DIR/server.py"

echo ""
echo "✓ HipCortex MCP server installed to $INSTALL_DIR/server.py"
echo ""
echo "Add to Cursor (.cursor/mcp.json):"
echo '  { "mcpServers": { "hipcortex": { "command": "python", "args": ["'"$INSTALL_DIR/server.py"'"], "env": { "HIPCORTEX_URL": "http://localhost:3030" } } } }'
echo ""
echo "Add to Claude Code (~/.claude/settings.json → mcpServers):"
echo '  "hipcortex": { "command": "python", "args": ["'"$INSTALL_DIR/server.py"'"], "env": { "HIPCORTEX_URL": "http://localhost:3030" } }'
```

- [ ] **Step 7: Create sdk/mcp/README.md**

```markdown
# HipCortex MCP Server

Expose HipCortex memory as tools for AI coding assistants.

**Supports:** Cursor · Claude Code · Windsurf · Zed AI · any MCP client

## Install (30 seconds)

```bash
curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/install.sh | bash
```

Or manually:
```bash
pip install requests
curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/server.py \
  -o ~/.hipcortex-mcp/server.py
```

## Connect to Cursor

Create or edit `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["~/.hipcortex-mcp/server.py"],
      "env": {
        "HIPCORTEX_URL": "http://localhost:3030"
      }
    }
  }
}
```

## Connect to Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["~/.hipcortex-mcp/server.py"],
      "env": {
        "HIPCORTEX_URL": "http://localhost:3030",
        "HIPCORTEX_API_KEY": "sk-your-key"
      }
    }
  }
}
```

Then restart Claude Code. You'll see the tools in the tool list.

## Available tools

| Tool | What it does |
|------|-------------|
| `add_memory` | Store a decision, finding, or code note |
| `search_memory` | Recall relevant past context by keyword |
| `forget_actor` | Delete all memories for a project scope |
| `get_stats` | Show memory store statistics |

## Usage example (in Cursor/Claude Code)

```
You: Remember that we chose JWT over session cookies because of our microservices architecture.

AI: [calls add_memory(actor="my-project", action="decided", target="JWT over session cookies — microservices require stateless auth")]
✓ Memory stored (id: abc-123)
```

Later:
```
You: How did we decide to handle authentication?

AI: [calls search_memory(query="authentication")]
• [decided] JWT over session cookies — microservices require stateless auth (score: 0.92)
```

## Start HipCortex server

```bash
# Option A: pre-built binary (no Rust needed)
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-arm64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# Option B: managed (free tier)
curl https://hipcortex.fly.dev/health  # already running — just use this URL
```
```

- [ ] **Step 8: Commit**

```bash
git add sdk/mcp/
git commit -m "feat: MCP server for Cursor, Claude Code, Windsurf

JSON-RPC 2.0 over stdio. Tools: add_memory, search_memory, forget_actor, get_stats.
Zero dependencies beyond Python stdlib + requests.
5 unit tests passing.

Install:
  curl -fsSL .../install.sh | bash

Cursor: .cursor/mcp.json → mcpServers.hipcortex
Claude Code: ~/.claude/settings.json → mcpServers.hipcortex"
```

---

## Final Integration

- [ ] **Run all Python tests**

```bash
cd sdk/python
pytest tests/ -v -k "not integration"
cd ../mcp
pytest test_server.py -v
```
Expected: all pass.

- [ ] **Run Rust build**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error|Finished"
```
Expected: `Finished dev profile`

- [ ] **Push + PR**

```bash
git push origin claude/pedantic-edison-28b84c
gh pr create --title "feat: golden path UX sprint (npm, async langchain, unified search, shutdown, MCP)" \
  --base main
```

- [ ] **Deploy to Fly.io**

```bash
fly deploy --app hipcortex --remote-only
```
