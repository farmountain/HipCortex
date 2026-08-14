# Cognitive OS Passive Integration — Design Spec

**Date:** 2026-08-15
**Status:** Approved for implementation
**Supersedes:** `2026-08-14-sdk-channel-gap-remediation.md` (plan) — passive layer added, sprint order revised

---

## 1. Problem Statement

HipCortex is positioned as a Cognitive Operating System (L0 substrate). A cognitive OS must capture agent state automatically — the same way an OS kernel manages memory without applications explicitly requesting every allocation.

The existing plan (`docs/superpowers/plans/2026-08-14-sdk-channel-gap-remediation.md`) closes the **explicit invocation layer** (75 missing API methods across 7 surfaces). This is necessary but insufficient.

**The missing dimension: passive observation.**

An agent that never calls `hipcortex_remember` currently captures nothing. Zero cognitive state. The OS produces no value unless explicitly invoked. This is the "memory API" failure mode — not a cognitive OS.

### Concrete failure scenario

```
CrewAI researcher agent runs 3-hour task:
  - Calls 40 tools
  - Makes 12 key decisions  
  - Encounters 3 errors and recovers
  - Produces final report

HipCortex memory after task: EMPTY
(agent never called hipcortex_remember)
```

This is unacceptable for a cognitive OS. It is acceptable for a memory API.

---

## 2. The Two-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        AI AGENT                                  │
│   (Claude Code, Cursor, CrewAI, AutoGen, LangChain, Codex)      │
└──────────────────┬───────────────────────────────────────────────┘
                   │
      ┌────────────┴────────────┐
      ▼                         ▼
┌─────────────┐         ┌──────────────────┐
│  EXPLICIT   │         │    PASSIVE        │
│  LAYER      │         │    LAYER          │
│             │         │                  │
│ Agent calls │         │ Auto-captured:   │
│ tools/APIs  │         │ • Callbacks      │
│ when it     │         │ • Lifecycle hooks │
│ chooses to  │         │ • Event listeners │
│             │         │ • MCP resources  │
└──────┬──────┘         └────────┬─────────┘
       └──────────┬──────────────┘
                  ▼
       ┌──────────────────────┐
       │  HipCortex REST API  │
       │  (Rust, :3030/:50051)│
       └──────────────────────┘
```

Both layers write to the same Rust server through the same REST routes. The passive layer is an adapter concern only — no Rust changes required.

---

## 3. Integration Profiles

Three profiles define what a user gets at each level of integration effort:

### Profile 0: Zero-Config
**Install HipCortex. Change 0 lines of agent code.**

Agent benefits automatically via passive hooks:
- Every LangChain tool call → auto-stored
- Every CrewAI task completion → auto-stored
- Every VS Code file save → auto-stored
- Every MCP session → context auto-injected

**What delivers this:** LangChain `BaseCallbackHandler`, CrewAI `CrewObserver`, VSIX listeners, MCP resources.

### Profile 1: Standard
**Install + plug in memory backend.**

Agent gets conversational memory + search:
- `chain = ConversationChain(llm=llm, memory=HipCortexMemory(client))`
- `tools = make_memory_tools()`
- Explicit recall/search when needed

**What delivers this:** Current `HipCortexMemory`, current `make_memory_tools()` + 75-gap additions.

### Profile 2: Power
**Full API access.**

Agent can use all cognitive capabilities:
- World model predictions
- Causal interventions
- Coherence checking
- Self-model gating
- Audit/regulatory compliance

**What delivers this:** Complete API coverage (75-gap plan).

**Sprint order:** Profile 0 first (passive), Profile 1 second (API completeness), Profile 2 last (power ops). Each profile is independently useful.

---

## 4. Passive Layer Components

### 4.1 LangChain `BaseCallbackHandler` (`langchain_memory.py`)

**What it captures:** tool invocations, tool results, agent decisions, LLM errors, chain start/end.

**Contract:** Implements LangChain's `BaseCallbackHandler` interface. Called by every LangChain chain automatically — zero agent code changes.

```python
class HipCortexCallbackHandler(BaseCallbackHandler):
    """Passive observer: auto-stores agent execution events without explicit calls.
    
    Usage (one line, never call add_memory again):
        chain = AgentExecutor(agent=agent, tools=tools,
                              callbacks=[HipCortexCallbackHandler(client, actor="my-agent")])
    """
    
    def __init__(self, client: HipCortexClient, actor: str = "langchain-agent"):
        self._client = client
        self._actor = actor
    
    def on_tool_start(self, serialized, input_str, **kwargs):
        self._client.add_memory(actor=self._actor, action="tool_start",
                                target=f"{serialized.get('name','?')}: {input_str[:200]}")
    
    def on_tool_end(self, output, **kwargs):
        self._client.add_memory(actor=self._actor, action="tool_result",
                                target=str(output)[:300])
    
    def on_tool_error(self, error, **kwargs):
        self._client.add_memory(actor=self._actor, action="tool_error",
                                target=str(error)[:200])
    
    def on_agent_finish(self, finish, **kwargs):
        self._client.add_memory(actor=self._actor, action="decided",
                                target=finish.return_values.get("output", "")[:400])
    
    def on_llm_error(self, error, **kwargs):
        self._client.add_memory(actor=self._actor, action="llm_error",
                                target=str(error)[:200])
    
    def on_chain_end(self, outputs, **kwargs):
        output = outputs.get("output", outputs.get("text", str(outputs)))
        self._client.add_memory(actor=self._actor, action="chain_result",
                                target=str(output)[:400])
```

**Acceptance criteria:**
- `HipCortexCallbackHandler` importable from `hipcortex.langchain_memory`
- `on_tool_end` stores record without explicit `add_memory` call
- `on_agent_finish` stores decision record
- Error in `add_memory` does NOT raise — must fail silently (passive observer must not break agent)
- `from_settings()` classmethod for zero-config instantiation

### 4.2 CrewAI `CrewObserver` + Pre-Kickoff Injection (`adapters/crewai.py`)

**What it captures:** task completions, agent tool use during tasks, crew-level results.

**Pre-kickoff injection (bidirectional):** Before `crew.kickoff()`, searches HipCortex for memories relevant to each agent's goal and injects them into the agent's `backstory`. This is the "context injection" half of passive integration.

```python
class HipCortexCrewObserver:
    """Attach to a Crew to passively capture all agent actions and inject context.

    CrewAI uses function-based callbacks, not class-based hooks. This class exposes
    `step_callback` and `task_callback` properties that return bound callables
    matching CrewAI's expected signatures.

    Usage:
        observer = HipCortexCrewObserver(client=client, actor_prefix="research")
        observer.inject_context(crew=my_crew)   # before kickoff

        my_crew = Crew(
            agents=[...], tasks=[...],
            step_callback=observer.step_callback,   # AgentAction → auto-captured
        )
        # Per-task callback: Task(description="...", callback=observer.task_callback)
        my_crew.kickoff()
    """

    def __init__(self, client: HipCortexClient, actor_prefix: str = "crew"):
        self._client = client
        self._actor_prefix = actor_prefix
        self._injected: set = set()  # guards idempotent inject_context

    def inject_context(self, crew, limit: int = 10) -> int:
        """Inject relevant memories into each agent's backstory. Idempotent."""
        injected = 0
        for agent in crew.agents:
            agent_id = id(agent)
            if agent_id in self._injected:
                continue
            try:
                memories = self._client.search(query=getattr(agent, "goal", ""), limit=limit)
                results = memories.get("results", [])
                if results:
                    context = "\n".join(
                        f"- {r['record'].get('target', '')}" for r in results
                    )
                    agent.backstory = (agent.backstory or "") + f"\n\nRelevant past context:\n{context}"
                    injected += len(results)
                self._injected.add(agent_id)
            except Exception:
                pass  # fail silently — passive observer must not block kickoff
        return injected

    @property
    def step_callback(self):
        """Returns callable for Crew(step_callback=...). Signature: fn(AgentAction) -> None."""
        def _step_cb(agent_action) -> None:
            try:
                tool = getattr(agent_action, "tool", None) or ""
                tool_input = getattr(agent_action, "tool_input", "") or ""
                actor = f"{self._actor_prefix}-step"
                self._client.add_memory(
                    actor=actor, action=f"used_{tool}" if tool else "acted",
                    target=str(tool_input)[:200]
                )
            except Exception:
                pass
        return _step_cb

    @property
    def task_callback(self):
        """Returns callable for Task(callback=...). Signature: fn(TaskOutput) -> None."""
        def _task_cb(task_output) -> None:
            try:
                output_str = getattr(task_output, "raw_output", str(task_output))
                self._client.add_memory(
                    actor=f"{self._actor_prefix}-task",
                    action="completed_task",
                    target=str(output_str)[:300]
                )
            except Exception:
                pass
        return _task_cb
```

**Acceptance criteria:**
- `HipCortexCrewObserver` importable from `hipcortex.adapters.crewai`
- `inject_context()` modifies agent backstories and returns count of injected memories
- `on_task_complete` stores record per completed task
- Errors fail silently
- `inject_context()` with no memories in HipCortex changes nothing (idempotent)

### 4.3 AutoGen `MessageObserver` (`adapters/autogen.py`)

**What it captures:** every message sent/received in a group chat or conversation.

```python
class HipCortexAutoGenObserver:
    """Passive observer for AutoGen GroupChat or ConversableAgent.
    
    Usage:
        observer = HipCortexAutoGenObserver(client=client, actor="research-team")
        # Wrap send() method or attach as message hook
    """
    
    def on_message_received(self, message: dict, sender: str, receiver: str):
        content = message.get("content", "")[:300]
        if content:
            self._client.add_memory(
                actor=f"{self._actor}.{sender}",
                action="said",
                target=content
            )
    
    def on_function_call(self, function_name: str, arguments: dict, result: str):
        self._client.add_memory(
            actor=self._actor,
            action=f"called_{function_name}",
            target=f"args={str(arguments)[:150]} result={result[:150]}"
        )
```

**Acceptance criteria:**
- `HipCortexAutoGenObserver` importable from `hipcortex.adapters.autogen`
- `on_message_received` stores record per message
- `on_function_call` stores function call + result
- Does not modify `HipCortexAutoGenMemory` (separate concern)

### 4.4 VSIX Passive Event Listeners (`vscode-extension/src/extension.ts`)

**What it captures:** file saves, terminal commands, active file changes in the IDE.

```typescript
// All registered in activate(), never called by agent explicitly

function registerPassiveListeners(context: vscode.ExtensionContext, client: HipCortexApiClient, actor: string) {
    // File saves
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(doc => {
            const rel = vscode.workspace.asRelativePath(doc.uri);
            client.addMemory({ actor, action: "saved_file", target: rel })
                  .catch(() => {}); // fail silently
        })
    );
    
    // Terminal output (captures commands run)
    context.subscriptions.push(
        vscode.window.onDidWriteTerminalData(event => {
            const data = event.data.trim();
            if (data && data.length > 3 && data.length < 500) {
                client.addMemory({ actor, action: "terminal", target: data })
                      .catch(() => {});
            }
        })
    );
    
    // Active editor changes (tracks what files are being worked on)
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(editor => {
            if (editor) {
                const rel = vscode.workspace.asRelativePath(editor.document.uri);
                client.addMemory({ actor, action: "opened_file", target: rel })
                      .catch(() => {});
            }
        })
    );
}
```

**Configuration:** Passive listeners are ON by default. User can disable via VS Code settings `hipcortex.passiveCapture: false`.

**Acceptance criteria:**
- `registerPassiveListeners()` called from `activate()` if `passiveCapture` setting is true (default: true)
- `onDidSaveTextDocument` fires → record created in HipCortex (verified in Phase 4 e2e)
- Listener errors never propagate to VS Code UI (all `.catch(() => {})`)
- `hipcortex.passiveCapture` setting added to `package.json` `contributes.configuration`
- Setting `false` skips listener registration

### 4.5 MCP Resources (Auto-Injected Context) (`sdk/mcp/server.py`)

**What it delivers:** Context injected automatically at session start in every MCP client (Claude Code, Cursor, Windsurf, etc.) without the LLM choosing to call a tool.

```python
@server.list_resources()
async def list_resources() -> list[Resource]:
    return [
        Resource(
            uri=AnyUrl("hipcortex://context/relevant"),
            name="Relevant Memory Context",
            description="Memories relevant to the current working directory and recent activity. Auto-injected at session start.",
            mimeType="text/plain",
        ),
        Resource(
            uri=AnyUrl("hipcortex://beliefs/current"),
            name="Current World State",
            description="Live beliefs, hypotheses, and world model state.",
            mimeType="application/json",
        ),
        Resource(
            uri=AnyUrl("hipcortex://context/conversation"),
            name="Conversation History",
            description="Recent conversation memories for this actor.",
            mimeType="text/plain",
        ),
    ]

@server.read_resource()
async def read_resource(uri: AnyUrl) -> str:
    uri_str = str(uri)
    if uri_str == "hipcortex://context/relevant":
        # Use git root or CWD as context hint
        import os
        cwd = os.getcwd()
        results = _post("/memory/search", {"query": os.path.basename(cwd), "limit": 10})
        memories = results.get("results", [])
        if not memories:
            return "No relevant memories found for current context."
        return "\n".join(f"- [{r['record'].get('action','?')}] {r['record'].get('target','')}"
                         for r in memories)
    
    elif uri_str == "hipcortex://beliefs/current":
        beliefs = _get("/memory/live_beliefs")
        import json
        return json.dumps(beliefs, indent=2)
    
    elif uri_str == "hipcortex://context/conversation":
        actor = os.environ.get("HIPCORTEX_ACTOR", "mcp-session")
        history = _post("/memory/query", {"actor": actor, "limit": 20})
        records = history.get("records", [])
        return "\n".join(f"[{r.get('action','?')}] {r.get('target','')}" for r in records)
    
    return "Resource not found"
```

**Acceptance criteria:**
- `list_resources()` returns 3 resources
- `read_resource("hipcortex://context/relevant")` returns non-empty string (or "No relevant memories" sentinel)
- `read_resource("hipcortex://beliefs/current")` returns valid JSON
- Resources available in MCP client's resource list (verified via `claude mcp list-resources`)
- Resource read errors return empty string, not exception (MCP client must not crash)

---

## 5. Sprint Order (Revised — Passive First)

```
Sprint 0: MCP Resources (G-R1) — fastest, highest leverage (auto-inject)
Sprint 1: LangChain CallbackHandler (G-P1) — dominant framework
Sprint 2: CrewAI CrewObserver + inject_context (G-P2)
Sprint 3: AutoGen MessageObserver (G-P3)
Sprint 4: VSIX passive listeners (G-P4)
Sprint 5: MCP tool completeness G1-G7 (from original plan)
Sprint 6: TypeScript SDK G8-G13
Sprint 7: Python async_client G14
Sprint 8: CrewAI tool expansion G15 (11 tools)
Sprint 9: LangChain Retriever + reflect G16
Sprint 10: VSIX LM tools G17
Sprint 11: Channel installers G18-G19
Sprint 12: E2E Phase 6 + react_validator.py G20
Sprint 13: Version bumps + capabilities matrix
```

**Gate between Sprint 4 and Sprint 5:** Run Profile 0 end-to-end test:
- Start CrewAI crew with HipCortex passive hooks, zero explicit memory calls
- Verify HipCortex contains >0 memories after crew completes
- If passes: proceed to explicit API completeness (Sprint 5+)
- If fails: fix passive layer before continuing

---

## 6. Cohesion Constraints

All constraints from the 7-framework design (`2026-08-14`) remain in force:

1. **One abstraction rule:** Every passive hook must call `self._client._post()` / `self._client._get()` — never raw `httpx`/`requests`. MCP resources must use `_post()` / `_get()` helpers.

2. **Fail silently:** Passive observers must never raise. If HipCortex server is down, agents continue working. Wrap every `add_memory` call in `try/except` in passive code.

3. **No duplicate HTTP clients:** VSIX listeners use existing `apiClient` singleton. CrewAI observer uses the `HipCortexClient` instance passed in. No new connection pools.

4. **Actor namespacing:** Passive captures use scoped actor IDs: `{prefix}-{agent_role}` for CrewAI, `{prefix}.{sender}` for AutoGen, configurable `defaultActor` for VSIX. No cross-actor pollution.

5. **Idempotent injection:** `CrewObserver.inject_context()` called twice must not double-inject backstory. Guard with a `_injected` flag.

---

## 7. Files Created/Modified

**Passive layer (Sprints 0–4):**
```
sdk/mcp/server.py               — add list_resources() + read_resource() handlers
sdk/python/hipcortex/langchain_memory.py  — add HipCortexCallbackHandler class
sdk/python/hipcortex/adapters/crewai.py   — add HipCortexCrewObserver class
sdk/python/hipcortex/adapters/autogen.py  — add HipCortexAutoGenObserver class
vscode-extension/src/extension.ts         — add registerPassiveListeners()
vscode-extension/package.json             — add hipcortex.passiveCapture config setting
```

**Explicit API completeness (Sprints 5–11):** all files from `2026-08-14-sdk-channel-gap-remediation.md` unchanged.

**Testing:**
```
tests/e2e_user_harness/suites/test_phase6_gap_coverage.py   — add passive layer tests
tests/e2e_user_harness/suites/test_phase3_framework_integrations.py — extend
tests/e2e_user_harness/suites/test_phase4_vscode_extension.py — extend
scripts/react_validator.py  — add passive AC IDs
```

---

## 8. Acceptance Criteria Summary (Passive Layer)

| ID | Test | Pass condition |
|----|------|---------------|
| AC-P0-1 | MCP resources listed | `list_resources` returns 3 resources |
| AC-P0-2 | Relevant context resource readable | Returns non-empty string |
| AC-P0-3 | Beliefs resource readable | Returns valid JSON |
| AC-P1-1 | CallbackHandler importable | `from hipcortex.langchain_memory import HipCortexCallbackHandler` |
| AC-P1-2 | `on_tool_end` passive capture | After chain run with callback, HipCortex has ≥1 tool_result record |
| AC-P1-3 | `on_agent_finish` passive capture | HipCortex has ≥1 decided record after agent finishes |
| AC-P1-4 | Fail silently | Server down + `on_tool_end` → no exception raised |
| AC-P2-1 | CrewObserver importable | `from hipcortex.adapters.crewai import HipCortexCrewObserver` |
| AC-P2-2 | `inject_context` modifies backstory | Agent backstory contains memory content after inject |
| AC-P2-3 | `inject_context` idempotent | Called twice → backstory injected once |
| AC-P2-4 | `on_task_complete` passive capture | HipCortex has record after task completion |
| AC-P3-1 | AutoGenObserver importable | `from hipcortex.adapters.autogen import HipCortexAutoGenObserver` |
| AC-P3-2 | `on_message_received` capture | HipCortex has ≥1 "said" record after message |
| AC-P4-1 | VSIX passive capture enabled | `passiveCapture: true` setting exists |
| AC-P4-2 | File save captured | Save file in VS Code → HipCortex has saved_file record |
| AC-P4-3 | Passive capture disableable | `passiveCapture: false` → no listener registered |
| AC-P5-1 | Profile 0 e2e test | CrewAI crew runs, 0 explicit memory calls, HipCortex has >0 records |
| AC-P5-2 | Memories not duplicated | Single tool call → single record (not N duplicates) |

---

## 9. What This Spec Does NOT Cover

- Rust server changes (none required for passive layer)
- LLM prompt engineering (not L0 concern)
- Rate limiting passive captures (post-v0.6.0 concern; current: capture all)
- Deduplication of identical captures (post-v0.6.0; current: allow duplicates)
- Privacy controls for VSIX captures (post-v0.6.0; current: passiveCapture toggle is sufficient)
