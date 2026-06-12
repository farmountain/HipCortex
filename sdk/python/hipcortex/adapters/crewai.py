"""CrewAI tool adapter for HipCortex.

Exposes HipCortex memory operations as CrewAI ``BaseTool`` subclasses
so agents can store and retrieve memories as tool calls during task execution.

Usage::

    from crewai import Agent, Task, Crew
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import (
        HipCortexRememberTool,
        HipCortexRecallTool,
        HipCortexForgetTool,
    )

    client = HipCortexClient(base_url="http://127.0.0.1:3030")
    tools = [
        HipCortexRememberTool(client=client, agent_id="researcher"),
        HipCortexRecallTool(client=client, agent_id="researcher"),
        HipCortexForgetTool(client=client, agent_id="researcher"),
    ]

    researcher = Agent(
        role="Senior Researcher",
        goal="Research and remember findings",
        tools=tools,
        ...
    )
"""

from __future__ import annotations

from typing import Any, Optional, Type

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
