# HipCortex Passive Integration Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a passive layer so HipCortex captures agent activity automatically — zero explicit `add_memory` calls required (Profile 0).

**Architecture:** Two-layer model — explicit layer (agent calls tools) already exists; this plan adds passive layer (auto-captured via framework hooks, MCP resources, VSIX listeners). Five passive adapters: MCP resources, LangChain callback handler, CrewAI crew observer, AutoGen observer, VSIX terminal listener. All observers are fail-silent (never raise, never break the agent).

**Tech Stack:** Python 3.10+, MCP stdio protocol, LangChain ≥0.1, CrewAI ≥0.2, AutoGen ≥0.4, TypeScript/VS Code Extension API, Axum web server (Rust), pytest, cargo test.

**Spec:** `docs/superpowers/specs/2026-08-15-cognitive-os-passive-integration-design.md` (commit `add4da9`)

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `sdk/mcp/server.py` | Modify | Add RESOURCES constant + handlers + capabilities update |
| `sdk/python/hipcortex/langchain_memory.py` | Modify | Add `HipCortexCallbackHandler` class |
| `sdk/python/hipcortex/adapters/crewai.py` | Modify | Add `HipCortexCrewObserver` class |
| `sdk/python/hipcortex/adapters/autogen.py` | Modify | Add `HipCortexAutoGenObserver` class |
| `vscode-extension/src/extension.ts` | Modify | Guard onSave + add terminal listener |
| `vscode-extension/package.json` | Modify | Add `hipcortex.passiveCapture` config property |
| `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py` | Create | Profile 0 E2E gate tests |
| `tests/e2e_user_harness/suites/test_phase7_passive_layer.py` | Create | Passive observer unit/integration tests |

---

### Task 0: MCP Resources (Profile 0 — Zero-Config for MCP agents)

MCP resources are auto-injected at session start by MCP hosts without any LLM tool call. Adding `resources/list` + `resources/read` makes HipCortex context available to any MCP-compatible agent automatically.

**Files:**
- Modify: `sdk/mcp/server.py` (lines 76–140 area for RESOURCES, lines 703–750 area for handlers, lines 766–795 dispatch loop)

- [ ] **Step 1: Write the failing test**

Create `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py`:

```python
"""Profile 0 / MCP resources gate tests."""
import json, subprocess, sys, os

PYTHON = sys.executable
SERVER = os.path.join(os.path.dirname(__file__), "../../../sdk/mcp/server.py")

def _send(proc, msg):
    line = json.dumps(msg) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

def _start_server():
    env = {**os.environ, "HIPCORTEX_URL": "http://localhost:8787",
           "HIPCORTEX_API_KEY": "test"}
    return subprocess.Popen(
        [PYTHON, SERVER],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        text=True, env=env
    )

def test_mcp_initialize_advertises_resources():
    proc = _start_server()
    try:
        resp = _send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
        caps = resp["result"]["capabilities"]
        assert "resources" in caps, f"capabilities missing 'resources': {caps}"
    finally:
        proc.terminate()

def test_mcp_resources_list_returns_three_resources():
    proc = _start_server()
    try:
        _send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
        resp = _send(proc, {"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}})
        resources = resp["result"]["resources"]
        assert len(resources) == 3, f"expected 3 resources, got {len(resources)}"
        uris = {r["uri"] for r in resources}
        assert "hipcortex://context/relevant" in uris
        assert "hipcortex://beliefs/current" in uris
        assert "hipcortex://context/conversation" in uris
    finally:
        proc.terminate()

def test_mcp_resource_read_returns_content_silently_on_server_error():
    """If HipCortex server unreachable, resource read returns empty content (not error)."""
    proc = _start_server()
    try:
        _send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
        resp = _send(proc, {
            "jsonrpc":"2.0","id":2,"method":"resources/read",
            "params":{"uri":"hipcortex://context/relevant"}
        })
        # Must return result (not error) even when backend unreachable
        assert "result" in resp, f"expected result, got: {resp}"
        assert "contents" in resp["result"]
    finally:
        proc.terminate()

def test_mcp_version_is_0_6_0():
    proc = _start_server()
    try:
        resp = _send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
        ver = resp["result"]["serverInfo"]["version"]
        assert ver == "0.6.0", f"expected 0.6.0, got {ver}"
    finally:
        proc.terminate()
```

- [ ] **Step 2: Run to verify all 4 tests fail**

```bash
cd D:/all_projects/hipcortex
python -m pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py -v
```

Expected: 4 FAILED (resources not in capabilities, resources/list unknown method, version wrong)

- [ ] **Step 3: Read current server.py structure (lines 60–85 and 750–795)**

```bash
# Verify exact line numbers before editing
grep -n "TOOLS\s*=\|def dispatch_tool\|def main\|\"initialize\"\|\"tools/list\"\|serverInfo\|0\.5\.2" sdk/mcp/server.py | head -30
```

- [ ] **Step 4: Add RESOURCES constant after TOOLS list**

In `sdk/mcp/server.py`, after the closing `]` of `TOOLS = [...]`, add:

```python
RESOURCES = [
    {
        "uri": "hipcortex://context/relevant",
        "name": "Relevant Memory Context",
        "description": "Top-k memories relevant to the current session, auto-injected.",
        "mimeType": "text/plain",
    },
    {
        "uri": "hipcortex://beliefs/current",
        "name": "Current Beliefs",
        "description": "Active belief records from HipCortex symbolic store.",
        "mimeType": "application/json",
    },
    {
        "uri": "hipcortex://context/conversation",
        "name": "Conversation History",
        "description": "Recent temporal memory traces for this session.",
        "mimeType": "text/plain",
    },
]
```

- [ ] **Step 5: Add resource handlers after `dispatch_tool` function**

In `sdk/mcp/server.py`, after `def dispatch_tool(name, args):` function body, add:

```python
def handle_resources_list():
    return {"resources": RESOURCES}


def handle_resource_read(uri: str):
    try:
        if uri == "hipcortex://context/relevant":
            data = _get("/memory/search?q=context&limit=5")
            lines = []
            for item in (data if isinstance(data, list) else []):
                actor = item.get("actor", "")
                action = item.get("action", "")
                target = item.get("target", "")
                lines.append(f"[{actor}] {action} → {target}")
            text = "\n".join(lines) if lines else "(no memories yet)"
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}
        elif uri == "hipcortex://beliefs/current":
            data = _get("/memory/search?q=belief&record_type=Symbolic&limit=10")
            return {"contents": [{"uri": uri, "mimeType": "application/json",
                                   "text": json.dumps(data if isinstance(data, list) else [])}]}
        elif uri == "hipcortex://context/conversation":
            actor = os.environ.get("HIPCORTEX_ACTOR", "mcp-session")
            data = _get(f"/memory/query?actor={actor}&limit=20")
            lines = []
            for item in (data if isinstance(data, list) else []):
                lines.append(f"{item.get('action','')} → {item.get('target','')}")
            text = "\n".join(lines) if lines else "(no history yet)"
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}
        else:
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": ""}]}
    except Exception:
        return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": ""}]}
```

- [ ] **Step 6: Update `initialize` response — add `"resources": {}` to capabilities and bump version**

Find this block in `main()`:

```python
"capabilities": {"tools": {}},
"serverInfo": {"name": "hipcortex", "version": "0.5.2"},
```

Replace with:

```python
"capabilities": {"tools": {}, "resources": {}},
"serverInfo": {"name": "hipcortex", "version": "0.6.0"},
```

- [ ] **Step 7: Add resources/list and resources/read cases to dispatch in `main()`**

After the `elif method == "tools/call":` block, add:

```python
elif method == "resources/list":
    respond(id_, handle_resources_list())
elif method == "resources/read":
    respond(id_, handle_resource_read(params.get("uri", "")))
```

- [ ] **Step 8: Run tests to verify all 4 pass**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py -v
```

Expected: 4 PASSED

- [ ] **Step 9: Commit**

```bash
git add sdk/mcp/server.py tests/e2e_user_harness/suites/test_phase6_gap_coverage.py
git commit -m "feat(mcp): add resources/list + resources/read, bump version to 0.6.0"
```

---

### Task 1: LangChain `HipCortexCallbackHandler` (Passive LangChain observer)

LangChain's `BaseCallbackHandler` is called by chains automatically — no agent code change needed. We add it alongside the existing `HipCortexMemory` class.

**Files:**
- Modify: `sdk/python/hipcortex/langchain_memory.py`

- [ ] **Step 1: Write the failing test**

Create `tests/e2e_user_harness/suites/test_phase7_passive_layer.py` (start of file):

```python
"""Passive observer unit tests."""
import sys, os, json
from unittest.mock import MagicMock, patch, call

# ── Task 1: LangChain callback handler ─────────────────────────────────────────

def test_langchain_callback_handler_importable():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    assert HipCortexCallbackHandler is not None

def test_langchain_callback_on_llm_start_calls_add_memory():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    mock_client = MagicMock()
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")
    handler.on_llm_start(serialized={}, prompts=["Hello world"], run_id="r1")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "lc-agent"
    assert kwargs["action"] == "llm_start"

def test_langchain_callback_on_llm_end_calls_add_memory():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    from langchain.schema import LLMResult, Generation
    mock_client = MagicMock()
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")
    result = LLMResult(generations=[[Generation(text="done")]])
    handler.on_llm_end(response=result, run_id="r1")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["action"] == "llm_end"

def test_langchain_callback_never_raises_on_client_error():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = RuntimeError("network down")
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")
    # Must NOT raise
    handler.on_llm_start(serialized={}, prompts=["test"], run_id="r1")
    handler.on_tool_start(serialized={"name": "search"}, input_str="q", run_id="r2")
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd D:/all_projects/hipcortex
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py::test_langchain_callback_handler_importable -v
```

Expected: ImportError or AttributeError

- [ ] **Step 3: Read langchain_memory.py imports section**

```bash
head -30 sdk/python/hipcortex/langchain_memory.py
```

- [ ] **Step 4: Add `HipCortexCallbackHandler` to `langchain_memory.py`**

At the end of the existing imports block, add:

```python
try:
    from langchain_core.callbacks.base import BaseCallbackHandler as _LCBaseCallbackHandler
    from langchain.schema import LLMResult as _LLMResult
    _LANGCHAIN_CALLBACKS_AVAILABLE = True
except ImportError:
    _LCBaseCallbackHandler = object
    _LLMResult = None
    _LANGCHAIN_CALLBACKS_AVAILABLE = False
```

Then at the end of the file, add:

```python
class HipCortexCallbackHandler(_LCBaseCallbackHandler):
    """Passive LangChain observer. Wire via: chain = MyChain(callbacks=[HipCortexCallbackHandler(client)])."""

    def __init__(self, client, actor: str = "langchain-agent", **kwargs):
        if _LANGCHAIN_CALLBACKS_AVAILABLE:
            super().__init__(**kwargs)
        self._client = client
        self._actor = actor

    def _capture(self, action: str, target: str, metadata: dict = None) -> None:
        try:
            self._client.add_memory(
                actor=self._actor,
                action=action,
                target=target,
                record_type="Temporal",
                source="langchain-passive",
                metadata=metadata or {},
            )
        except Exception:
            pass

    def on_llm_start(self, serialized, prompts, **kwargs) -> None:
        snippet = (prompts[0][:120] if prompts else "") if prompts else ""
        self._capture("llm_start", snippet)

    def on_llm_end(self, response, **kwargs) -> None:
        try:
            text = response.generations[0][0].text[:120] if response.generations else ""
        except Exception:
            text = ""
        self._capture("llm_end", text)

    def on_tool_start(self, serialized, input_str, **kwargs) -> None:
        tool_name = (serialized or {}).get("name", "unknown-tool")
        self._capture("tool_start", f"{tool_name}({str(input_str)[:80]})")

    def on_tool_end(self, output, **kwargs) -> None:
        self._capture("tool_end", str(output)[:120])

    def on_agent_action(self, action, **kwargs) -> None:
        try:
            self._capture("agent_action", f"{action.tool}({str(action.tool_input)[:80]})")
        except Exception:
            pass

    def on_agent_finish(self, finish, **kwargs) -> None:
        try:
            self._capture("agent_finish", str(finish.return_values)[:120])
        except Exception:
            pass
```

- [ ] **Step 5: Run tests to verify all 4 pass**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "langchain" -v
```

Expected: 4 PASSED

- [ ] **Step 6: Commit**

```bash
git add sdk/python/hipcortex/langchain_memory.py tests/e2e_user_harness/suites/test_phase7_passive_layer.py
git commit -m "feat(langchain): add HipCortexCallbackHandler passive observer"
```

---

### Task 2: CrewAI `HipCortexCrewObserver` (Passive CrewAI observer)

CrewAI uses function-based callbacks: `Crew(step_callback=fn)` and `Task(callback=fn)`. The observer provides properties that return bound callables with the correct signatures.

**Files:**
- Modify: `sdk/python/hipcortex/adapters/crewai.py`

- [ ] **Step 1: Append failing tests to test_phase7_passive_layer.py**

```python
# ── Task 2: CrewAI crew observer ───────────────────────────────────────────────

def test_crewai_observer_importable():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    assert HipCortexCrewObserver is not None

def test_crewai_observer_step_callback_is_callable():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    cb = obs.step_callback
    assert callable(cb), "step_callback must return a callable"

def test_crewai_observer_step_callback_captures_action():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    action = MagicMock()
    action.tool = "SearchTool"
    action.tool_input = "python memory"
    action.result = "found docs"
    obs.step_callback(action)
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "crew-test"
    assert kwargs["action"] == "crew_step"
    assert "SearchTool" in kwargs["target"]

def test_crewai_observer_step_callback_never_raises():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = ConnectionError("timeout")
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    # Must NOT raise even when client fails
    obs.step_callback(MagicMock())

def test_crewai_observer_inject_context_idempotent():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    mock_client.query_memory.return_value = []
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    mock_crew = MagicMock()
    obs.inject_context(mock_crew)
    obs.inject_context(mock_crew)  # second call — must not double-inject
    assert mock_client.query_memory.call_count == 1, "inject_context must be idempotent"
```

- [ ] **Step 2: Run to verify tests fail**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "crewai" -v
```

Expected: ImportError on `HipCortexCrewObserver`

- [ ] **Step 3: Read current crewai.py to find insertion point**

```bash
tail -20 sdk/python/hipcortex/adapters/crewai.py
```

- [ ] **Step 4: Add `HipCortexCrewObserver` to crewai.py**

At the end of `sdk/python/hipcortex/adapters/crewai.py`, append:

```python
class HipCortexCrewObserver:
    """Passive CrewAI observer. Wire via: Crew(step_callback=obs.step_callback, task_callback=obs.task_callback)."""

    def __init__(self, client, actor: str = "crew-agent"):
        self._client = client
        self._actor = actor
        self._injected: set = set()

    def inject_context(self, crew) -> None:
        """Inject relevant memories into crew context before kickoff. Idempotent."""
        crew_id = id(crew)
        if crew_id in self._injected:
            return
        self._injected.add(crew_id)
        try:
            memories = self._client.query_memory(actor=self._actor, limit=5)
            if memories and hasattr(crew, "context"):
                snippets = [f"[{m.get('action','')}] {m.get('target','')}" for m in memories]
                prefix = "Prior context:\n" + "\n".join(snippets)
                if isinstance(crew.context, str):
                    crew.context = prefix + "\n\n" + crew.context
        except Exception:
            pass

    @property
    def step_callback(self):
        """Returns callable for Crew(step_callback=...). Signature: fn(AgentAction) -> None."""
        def _on_step(action) -> None:
            try:
                tool = getattr(action, "tool", "unknown")
                tool_input = str(getattr(action, "tool_input", ""))[:80]
                self._client.add_memory(
                    actor=self._actor,
                    action="crew_step",
                    target=f"{tool}({tool_input})",
                    record_type="Temporal",
                    source="crewai-passive",
                )
            except Exception:
                pass
        return _on_step

    @property
    def task_callback(self):
        """Returns callable for Task(callback=...). Signature: fn(TaskOutput) -> None."""
        def _on_task(output) -> None:
            try:
                raw = getattr(output, "raw_output", None) or str(output)
                self._client.add_memory(
                    actor=self._actor,
                    action="task_complete",
                    target=str(raw)[:120],
                    record_type="Reflexion",
                    source="crewai-passive",
                )
            except Exception:
                pass
        return _on_task
```

- [ ] **Step 5: Run tests to verify all 5 crewai tests pass**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "crewai" -v
```

Expected: 5 PASSED

- [ ] **Step 6: Commit**

```bash
git add sdk/python/hipcortex/adapters/crewai.py tests/e2e_user_harness/suites/test_phase7_passive_layer.py
git commit -m "feat(crewai): add HipCortexCrewObserver passive observer"
```

---

### Task 3: AutoGen `HipCortexAutoGenObserver` (Passive AutoGen observer)

AutoGen 0.4 has a `Memory` protocol (already implemented). We add a separate `HipCortexAutoGenObserver` that wraps v0.3 send/receive hooks for passive capture without requiring Memory protocol adoption.

**Files:**
- Modify: `sdk/python/hipcortex/adapters/autogen.py`

- [ ] **Step 1: Append failing tests**

```python
# ── Task 3: AutoGen observer ───────────────────────────────────────────────────

def test_autogen_observer_importable():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    assert HipCortexAutoGenObserver is not None

def test_autogen_observer_on_message_received_captures():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_message_received(sender="UserProxy", content="What is 2+2?", role="user")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "autogen-agent"
    assert kwargs["action"] == "message_received"
    assert "What is 2+2?" in kwargs["target"]

def test_autogen_observer_on_function_call_captures():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_function_call(name="web_search", arguments={"query": "python"})
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["action"] == "function_call"
    assert "web_search" in kwargs["target"]

def test_autogen_observer_never_raises():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = OSError("socket error")
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_message_received(sender="A", content="hello", role="user")  # must not raise
    obs.on_function_call(name="fn", arguments={})  # must not raise

def test_autogen_observer_v03_send_hook():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    hook = obs.make_v03_send_hook()
    hook({"content": "Hello there", "role": "assistant"})
    mock_client.add_memory.assert_called_once()
```

- [ ] **Step 2: Run to verify tests fail**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "autogen" -v
```

Expected: ImportError on `HipCortexAutoGenObserver`

- [ ] **Step 3: Read current autogen.py tail**

```bash
tail -20 sdk/python/hipcortex/adapters/autogen.py
```

- [ ] **Step 4: Add `HipCortexAutoGenObserver` to autogen.py**

At the end of `sdk/python/hipcortex/adapters/autogen.py`, append:

```python
class HipCortexAutoGenObserver:
    """Passive AutoGen observer. Wire via agent hooks or v0.3 send/receive hooks."""

    def __init__(self, client, actor: str = "autogen-agent"):
        self._client = client
        self._actor = actor

    def _capture(self, action: str, target: str, record_type: str = "Temporal") -> None:
        try:
            self._client.add_memory(
                actor=self._actor,
                action=action,
                target=target[:120],
                record_type=record_type,
                source="autogen-passive",
            )
        except Exception:
            pass

    def on_message_received(self, sender: str, content: str, role: str = "user") -> None:
        self._capture("message_received", f"[{sender}/{role}] {str(content)[:100]}")

    def on_message_sent(self, recipient: str, content: str) -> None:
        self._capture("message_sent", f"[→{recipient}] {str(content)[:100]}")

    def on_function_call(self, name: str, arguments: dict) -> None:
        args_str = str(arguments)[:80]
        self._capture("function_call", f"{name}({args_str})")

    def on_function_result(self, name: str, result: str) -> None:
        self._capture("function_result", f"{name} → {str(result)[:80]}")

    def make_v03_send_hook(self):
        """Returns hook for AutoGen v0.3 register_reply or send hook."""
        def _hook(message: dict) -> None:
            try:
                content = message.get("content", "") or ""
                role = message.get("role", "unknown")
                self._capture("message_sent", f"[{role}] {str(content)[:100]}")
            except Exception:
                pass
        return _hook

    def make_v03_receive_hook(self):
        """Returns hook for AutoGen v0.3 receive hook."""
        def _hook(message: dict, sender=None) -> None:
            try:
                content = message.get("content", "") or ""
                sender_name = getattr(sender, "name", str(sender)) if sender else "unknown"
                self._capture("message_received", f"[{sender_name}] {str(content)[:100]}")
            except Exception:
                pass
        return _hook
```

- [ ] **Step 5: Run tests to verify all 5 autogen tests pass**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "autogen" -v
```

Expected: 5 PASSED

- [ ] **Step 6: Commit**

```bash
git add sdk/python/hipcortex/adapters/autogen.py tests/e2e_user_harness/suites/test_phase7_passive_layer.py
git commit -m "feat(autogen): add HipCortexAutoGenObserver passive observer"
```

---

### Task 4: VSIX — `passiveCapture` config toggle + terminal listener

The `onSave` listener already exists at `extension.ts:1844`. This task adds: (a) a `passiveCapture` config guard so users can opt out, and (b) an `onDidWriteTerminalData` listener to capture terminal output passively.

**Files:**
- Modify: `vscode-extension/src/extension.ts`
- Modify: `vscode-extension/package.json`

- [ ] **Step 1: Append failing tests to test_phase7_passive_layer.py**

Note: VSIX TypeScript tests are compile-time verified via `tsc --noEmit`. Add a minimal comment block marker in the test file:

```python
# ── Task 4: VSIX passiveCapture config ─────────────────────────────────────────

def test_vsix_package_json_has_passive_capture_config():
    import json, os
    pkg_path = os.path.join(
        os.path.dirname(__file__),
        "../../../vscode-extension/package.json"
    )
    with open(pkg_path) as f:
        pkg = json.load(f)
    props = (pkg.get("contributes", {})
                .get("configuration", {})
                .get("properties", {}))
    assert "hipcortex.passiveCapture" in props, \
        "package.json missing hipcortex.passiveCapture config"
    config = props["hipcortex.passiveCapture"]
    assert config["type"] == "boolean"
    assert config["default"] is True

def test_vsix_extension_ts_has_passive_capture_guard():
    import os
    ts_path = os.path.join(
        os.path.dirname(__file__),
        "../../../vscode-extension/src/extension.ts"
    )
    with open(ts_path) as f:
        content = f.read()
    assert "passiveCapture" in content, \
        "extension.ts missing passiveCapture guard"
    assert "onDidWriteTerminalData" in content, \
        "extension.ts missing terminal listener"
```

- [ ] **Step 2: Run to verify tests fail**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "vsix" -v
```

Expected: 2 FAILED (hipcortex.passiveCapture not in package.json, passiveCapture not in extension.ts)

- [ ] **Step 3: Add `hipcortex.passiveCapture` to package.json**

Read the `contributes.configuration.properties` section of `vscode-extension/package.json` and add after the last existing property:

```json
"hipcortex.passiveCapture": {
    "type": "boolean",
    "default": true,
    "description": "When enabled, HipCortex automatically captures file saves and terminal output as memories. Disable to opt out of passive capture."
}
```

- [ ] **Step 4: Add passiveCapture guard to existing onSave listener**

In `extension.ts`, locate line 1844 which starts the `onSave` handler:

```typescript
const onSave = vscode.workspace.onDidSaveTextDocument(async (doc) => {
    if (doc.uri.scheme !== 'file') { return; }
```

After the `if (doc.uri.scheme !== 'file') { return; }` line, add:

```typescript
    const passiveCapture = vscode.workspace.getConfiguration('hipcortex').get<boolean>('passiveCapture', true);
    if (!passiveCapture) { return; }
```

- [ ] **Step 5: Add terminal listener after the onSave subscription**

Find where `context.subscriptions.push(onSave)` is called (approximately line 1902). After that push, add:

```typescript
const onTerminalData = vscode.window.onDidWriteTerminalData(async (e) => {
    const passiveCaptureTerminal = vscode.workspace.getConfiguration('hipcortex').get<boolean>('passiveCapture', true);
    if (!passiveCaptureTerminal) { return; }
    const data = e.data.replace(/\x1b\[[0-9;]*[mGKH]/g, '').trim();
    if (!data || data.length < 10) { return; }
    const snippet = data.slice(0, 200);
    try {
        const api = new HipCortexAPI();
        await api.addMemory({
            actor: 'vscode-terminal',
            action: 'terminal_output',
            target: snippet,
            record_type: 'Temporal',
            source: 'vscode-passive',
        });
    } catch (_) {
        // fail silently — never break the terminal
    }
});
context.subscriptions.push(onTerminalData);
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd vscode-extension
npx tsc --noEmit
```

Expected: 0 errors

- [ ] **Step 7: Run tests to verify 2 vsix tests pass**

```bash
cd D:/all_projects/hipcortex
python -m pytest tests/e2e_user_harness/suites/test_phase7_passive_layer.py -k "vsix" -v
```

Expected: 2 PASSED

- [ ] **Step 8: Commit**

```bash
git add vscode-extension/src/extension.ts vscode-extension/package.json tests/e2e_user_harness/suites/test_phase7_passive_layer.py
git commit -m "feat(vsix): add passiveCapture config + terminal data listener"
```

---

### Task 5: Profile 0 E2E Gate Tests (end-to-end passive capture validation)

Profile 0 acceptance criterion: zero explicit `add_memory` calls → HipCortex has >0 records. These tests validate the passive layer works against a live HipCortex server.

**Files:**
- Modify: `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py` (append)

- [ ] **Step 1: Append Profile 0 gate tests**

```python
# ── Profile 0 gate tests (requires live server at HIPCORTEX_URL) ───────────────

import os as _os, time as _time

_HIPCORTEX_URL = _os.environ.get("HIPCORTEX_URL", "http://localhost:8787")
_SKIP_LIVE = not _os.environ.get("HIPCORTEX_LIVE_TESTS", "")


def _requires_live(fn):
    import pytest
    return pytest.mark.skipif(
        _SKIP_LIVE,
        reason="Set HIPCORTEX_LIVE_TESTS=1 to run live server tests"
    )(fn)


@_requires_live
def test_profile0_langchain_passive_no_explicit_add():
    """Profile 0: LangChain callback captures without any explicit add_memory."""
    import requests
    from hipcortex import HipCortexClient
    from hipcortex.langchain_memory import HipCortexCallbackHandler

    client = HipCortexClient(base_url=_HIPCORTEX_URL)
    actor = f"profile0-lc-{int(_time.time())}"
    handler = HipCortexCallbackHandler(client=client, actor=actor)

    # Simulate what LangChain does automatically
    handler.on_llm_start(serialized={}, prompts=["Profile 0 test prompt"], run_id="p0")
    _time.sleep(0.2)

    resp = requests.get(f"{_HIPCORTEX_URL}/memory/query", params={"actor": actor, "limit": 10})
    records = resp.json()
    assert len(records) > 0, \
        f"Profile 0 FAIL: LangChain passive handler produced 0 records for actor={actor}"


@_requires_live
def test_profile0_crewai_passive_no_explicit_add():
    """Profile 0: CrewAI step_callback captures without any explicit add_memory."""
    import requests
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import HipCortexCrewObserver

    client = HipCortexClient(base_url=_HIPCORTEX_URL)
    actor = f"profile0-crew-{int(_time.time())}"
    obs = HipCortexCrewObserver(client=client, actor=actor)

    # Simulate what CrewAI does automatically via step_callback
    mock_action = MagicMock()
    mock_action.tool = "PassiveTestTool"
    mock_action.tool_input = "test query"
    obs.step_callback(mock_action)
    _time.sleep(0.2)

    resp = requests.get(f"{_HIPCORTEX_URL}/memory/query", params={"actor": actor, "limit": 10})
    records = resp.json()
    assert len(records) > 0, \
        f"Profile 0 FAIL: CrewAI passive observer produced 0 records for actor={actor}"
```

- [ ] **Step 2: Verify test structure is valid**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py --collect-only
```

Expected: 6 tests collected (4 MCP tests + 2 Profile 0 gate tests, both skipped without HIPCORTEX_LIVE_TESTS)

- [ ] **Step 3: Run existing tests still pass**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py -v -k "mcp"
```

Expected: 4 PASSED (the MCP resource tests from Task 0)

- [ ] **Step 4: Commit**

```bash
git add tests/e2e_user_harness/suites/test_phase6_gap_coverage.py
git commit -m "test(e2e): add Profile 0 passive capture gate tests"
```

---

### Task 6: Register Tests + Version Bumps

Register new test files in the harness and update version strings across SDK surfaces.

**Files:**
- Check: `tests/e2e_user_harness/suites/__init__.py` or test runner config
- Modify: `sdk/python/setup.py` or `sdk/python/pyproject.toml` — version `0.5.x` → `0.6.0`
- Modify: `sdk/mcp/server.py` — already done in Task 0
- Modify: `vscode-extension/package.json` — version bump

- [ ] **Step 1: Check how e2e harness discovers test suites**

```bash
ls tests/e2e_user_harness/suites/
cat tests/e2e_user_harness/suites/__init__.py 2>/dev/null || echo "no __init__.py"
grep -r "test_phase" tests/e2e_user_harness/ --include="*.py" -l
```

- [ ] **Step 2: Ensure new test files are discoverable**

If the harness uses explicit imports (e.g., `__init__.py` or a conftest), add imports for the two new suites:

```python
from . import test_phase6_gap_coverage
from . import test_phase7_passive_layer
```

If it uses pytest auto-discovery (conftest.py or pytest.ini with testpaths), no action needed — verify:

```bash
python -m pytest tests/e2e_user_harness/ --collect-only 2>&1 | grep "test_phase6\|test_phase7"
```

Expected: both suites appear in collection

- [ ] **Step 3: Bump Python SDK version**

Find version in `sdk/python/setup.py` or `sdk/python/pyproject.toml`:

```bash
grep -n "version" sdk/python/setup.py sdk/python/pyproject.toml 2>/dev/null | head -10
```

Update version string from current (`0.5.x`) to `0.6.0`.

- [ ] **Step 4: Bump VSIX version**

In `vscode-extension/package.json`, find `"version"` field and update to `0.6.0`.

- [ ] **Step 5: Run full passive layer test suite**

```bash
python -m pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py tests/e2e_user_harness/suites/test_phase7_passive_layer.py -v
```

Expected: All unit/MCP tests PASSED, live gate tests SKIPPED (no HIPCORTEX_LIVE_TESTS set)

- [ ] **Step 6: Final commit**

```bash
git add sdk/python/setup.py sdk/python/pyproject.toml vscode-extension/package.json
git commit -m "chore: bump SDK and VSIX version to 0.6.0 for passive integration release"
```

---

## Self-Review Checklist

**Spec coverage:**

| Spec requirement | Covered in task |
|---|---|
| MCP resources/list + resources/read | Task 0 |
| resources capability in initialize | Task 0 Step 6 |
| Version bump to 0.6.0 | Task 0 Step 6 + Task 6 |
| LangChain `HipCortexCallbackHandler` | Task 1 |
| Fail-silent (try/except: pass) in all observers | Tasks 1–4 (explicit in each `_capture`) |
| CrewAI `HipCortexCrewObserver` with `step_callback` property | Task 2 |
| CrewAI `inject_context` idempotent (set guard) | Task 2 Step 4 |
| AutoGen `HipCortexAutoGenObserver` + v0.3 hooks | Task 3 |
| VSIX `passiveCapture` toggle in package.json | Task 4 Step 3 |
| VSIX `onSave` guard with passiveCapture check | Task 4 Step 4 |
| VSIX `onDidWriteTerminalData` listener | Task 4 Step 5 |
| Profile 0 gate: zero explicit calls → >0 records | Task 5 |
| Profile 0 tests for LangChain + CrewAI | Task 5 |

**No placeholders found.** All steps contain exact code.

**Type consistency:** `_capture()` method name consistent across Tasks 1–3. `actor` parameter used consistently. `add_memory` call signature matches existing SDK pattern.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-15-passive-integration-plan.md`.

**Two execution options:**

**1. Subagent-Driven (recommended)** — fresh subagent per task, spec + quality review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session using executing-plans skill, batch with checkpoints.

Which approach?
