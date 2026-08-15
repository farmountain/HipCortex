"""CrewAI tool adapter for HipCortex.

Exposes HipCortex memory operations as CrewAI ``BaseTool`` subclasses
so agents can store and retrieve memories as tool calls during task execution.

Usage::

    from crewai import Agent, Task, Crew
    from hipcortex.adapters.crewai import make_memory_tools

    # Package-first: client + agent_id from hipcortex config
    tools = make_memory_tools()

    # Or explicit:
    # tools = make_memory_tools(client=client, agent_id="researcher")

    researcher = Agent(
        role="Senior Researcher",
        goal="Research and remember findings",
        tools=tools,
        ...
    )
"""

from __future__ import annotations

from typing import Any, List, Optional, Type

from ..client import HipCortexClient

try:
    from crewai_tools import BaseTool
    from pydantic import BaseModel, Field
    _CREWAI_AVAILABLE = True
except ImportError:
    _CREWAI_AVAILABLE = False

    class BaseModel:  # type: ignore[no-redef]
        pass

    class Field:  # type: ignore[no-redef]
        def __init__(self, *a: Any, **kw: Any) -> None: ...

    class BaseTool:  # type: ignore[no-redef]
        name: str = ""
        description: str = ""
        args_schema: Any = None
        def _run(self, *a: Any, **kw: Any) -> str: ...


# ---------------------------------------------------------------------------
# Input schemas
# ---------------------------------------------------------------------------

if _CREWAI_AVAILABLE:
    class RememberInput(BaseModel):
        content: str = Field(..., description="Observation or fact to remember.")
        action: str = Field(default="observation", description="Action tag (e.g. 'finding', 'decision').")

    class RecallInput(BaseModel):
        limit: int = Field(default=20, description="Max number of memories to return.")

    class ForgetInput(BaseModel):
        confirm: bool = Field(default=False, description="Set to True to confirm deletion.")


# ---------------------------------------------------------------------------
# Tools
# ---------------------------------------------------------------------------

class HipCortexRememberTool(BaseTool):  # type: ignore[misc]
    """CrewAI tool: store an observation in HipCortex memory."""

    name: str = "hipcortex_remember"
    description: str = (
        "Store an important observation, finding, or decision in persistent memory. "
        "Use this whenever you discover information worth remembering for future tasks."
    )
    if _CREWAI_AVAILABLE:
        args_schema: Type[BaseModel] = RememberInput

    def __init__(self, client: HipCortexClient, agent_id: str = "crewai-agent") -> None:
        super().__init__()
        self._client = client
        self._agent_id = agent_id

    def _run(self, content: str, action: str = "observation") -> str:
        result = self._client.add_memory(
            actor=self._agent_id,
            action=action,
            target=content,
            record_type="Temporal",
        )
        if result.get("success"):
            return f"Stored memory with id={result.get('record_id', '?')}"
        return f"Failed to store memory: {result.get('error', 'unknown error')}"


class HipCortexRecallTool(BaseTool):  # type: ignore[misc]
    """CrewAI tool: retrieve recent memories from HipCortex."""

    name: str = "hipcortex_recall"
    description: str = (
        "Retrieve recent memories and observations stored in HipCortex. "
        "Use this to recall past findings before starting a new research task."
    )
    if _CREWAI_AVAILABLE:
        args_schema: Type[BaseModel] = RecallInput

    def __init__(self, client: HipCortexClient, agent_id: str = "crewai-agent") -> None:
        super().__init__()
        self._client = client
        self._agent_id = agent_id

    def _run(self, limit: int = 20) -> str:
        records = self._client.get_conversation_history(self._agent_id, limit=limit)
        if not records:
            return "No memories found."
        records.sort(key=lambda r: r.get("timestamp", ""))
        lines = []
        for i, rec in enumerate(records, 1):
            lines.append(f"{i}. [{rec.get('action', '?')}] {rec.get('target', '')}")
        return "\n".join(lines)


class HipCortexForgetTool(BaseTool):  # type: ignore[misc]
    """CrewAI tool: delete all memories for this agent (GDPR / session reset)."""

    name: str = "hipcortex_forget"
    description: str = (
        "Delete all memories stored for this agent. "
        "Only use this when explicitly asked to clear memory or start fresh."
    )
    if _CREWAI_AVAILABLE:
        args_schema: Type[BaseModel] = ForgetInput

    def __init__(self, client: HipCortexClient, agent_id: str = "crewai-agent") -> None:
        super().__init__()
        self._client = client
        self._agent_id = agent_id

    def _run(self, confirm: bool = False) -> str:
        if not confirm:
            return "Deletion not confirmed. Pass confirm=True to proceed."
        result = self._client.forget(self._agent_id)
        deleted = result.get("records_deleted", 0)
        return f"Deleted {deleted} memory records for agent '{self._agent_id}'."


def make_memory_tools(
    client: Optional[HipCortexClient] = None,
    agent_id: Optional[str] = None,
) -> List[Any]:
    """Factory: remember / recall / forget tools with settings defaults.

    Args:
        client: Optional client; defaults to :func:`client_from_settings`.
        agent_id: Optional actor; defaults to :func:`default_session_id`.

    Returns:
        List of three CrewAI-compatible tools sharing the same client/actor.
    """
    from .common import client_from_settings, default_session_id

    if client is None:
        client = client_from_settings()
    if agent_id is None:
        agent_id = default_session_id()
    return [
        HipCortexRememberTool(client=client, agent_id=agent_id),
        HipCortexRecallTool(client=client, agent_id=agent_id),
        HipCortexForgetTool(client=client, agent_id=agent_id),
    ]


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
                snippets = [f"[{m.get('action', '')}] {m.get('target', '')}" for m in memories]
                prefix = "Prior context:\n" + "\n".join(snippets)
                if isinstance(crew.context, str):
                    crew.context = prefix + "\n\n" + crew.context
        except Exception:
            pass

    @property
    def step_callback(self):
        """Callable for Crew(step_callback=...). Signature: fn(AgentAction) -> None."""
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
        """Callable for Task(callback=...). Signature: fn(TaskOutput) -> None."""
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
