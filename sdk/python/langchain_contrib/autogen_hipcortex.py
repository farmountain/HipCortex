"""autogen_agentchat.memory.HipCortexMemory

Formatted for upstream contribution to microsoft/autogen (autogen-agentchat package).

Target path:
  python/packages/autogen-agentchat/autogen_agentchat/memory/hipcortex.py

PR description:
  Adds HipCortex (https://github.com/farmountain/HipCortex) as a
  persistent Memory implementation for AutoGen agents.
  HipCortex stores messages with temporal decay, causal world-modeling,
  a Merkle-chained audit log, and a GDPR right-to-forget REST endpoint.
  Self-hostable with zero external dependencies (single Rust binary).

Compatibility:
  autogen-agentchat >= 0.4.0
  autogen-core >= 0.4.0
  Python >= 3.10

Install:
  pip install hipcortex
"""

from __future__ import annotations

from typing import Any, List, Optional, Sequence

# AutoGen 0.4 memory protocol
try:
    from autogen_agentchat.base import Memory, MemoryQueryResult, UpdateContextResult
    from autogen_core import CancellationToken
    from autogen_core.memory import MemoryContent, MemoryMimeType
    _AUTOGEN_AVAILABLE = True
except ImportError:
    _AUTOGEN_AVAILABLE = False
    # Stubs for standalone use / testing
    class Memory:  # type: ignore[no-redef]
        async def query(self, *a, **kw): ...
        async def update_context(self, *a, **kw): ...
        async def add(self, *a, **kw): ...
        async def clear(self) -> None: ...
    class CancellationToken: pass  # type: ignore[no-redef]


class HipCortexMemory(Memory):
    """AutoGen ``Memory`` implementation backed by HipCortex REST API.

    Stores and retrieves agent messages using HipCortex's persistent memory
    engine — combining temporal decay, symbolic knowledge graphs, and causal
    world-model coherence checking into a single memory backend.

    Args:
        agent_id:   Scopes all memory to this agent/session identifier.
        url:        HipCortex server base URL (default: localhost:3030).
        api_key:    Optional ``X-Api-Key`` for hosted/managed SaaS tier.
        top_k:      Max memories returned per query (default: 10).
    """

    def __init__(
        self,
        agent_id: str = "autogen-agent",
        url: str = "http://localhost:3030",
        api_key: Optional[str] = None,
        top_k: int = 10,
    ) -> None:
        try:
            from hipcortex import HipCortexClient  # type: ignore[import]
        except ImportError as exc:
            raise ImportError(
                "hipcortex package required: pip install hipcortex"
            ) from exc

        self._client = HipCortexClient(base_url=url)
        if api_key:
            self._client._session.headers["X-Api-Key"] = api_key
        self._agent_id = agent_id
        self._top_k = top_k

    # ------------------------------------------------------------------
    # AutoGen Memory protocol
    # ------------------------------------------------------------------

    async def add(
        self,
        content: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> None:
        """Persist a memory content item."""
        if _AUTOGEN_AVAILABLE and hasattr(content, "content"):
            text    = str(content.content)
            mime    = getattr(content, "mime_type", None)
            action  = "ai_message" if (mime and "assistant" in str(mime).lower()) else "observation"
        else:
            text   = str(content)
            action = "observation"
        self._client.add_memory(
            actor=self._agent_id,
            action=action,
            target=text,
            record_type="Temporal",
        )

    async def query(
        self,
        query: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> Any:
        """Retrieve relevant memories for a query string or message."""
        query_text = str(getattr(query, "content", query))

        # Use semantic search endpoint for ranked results
        try:
            raw = self._client._session.post(
                f"{self._client.base_url}/memory/search",
                json={"query": query_text, "limit": self._top_k},
                timeout=self._client.timeout,
            )
            raw.raise_for_status()
            results = raw.json().get("results", [])
        except Exception:
            # Fallback: unranked actor query
            results = [
                {"record": r, "score": 1.0}
                for r in self._client.get_conversation_history(self._agent_id, limit=self._top_k)
            ]

        if not _AUTOGEN_AVAILABLE:
            return results

        # Wrap in AutoGen MemoryQueryResult
        memories = []
        for item in results:
            rec = item.get("record", item)
            memories.append(
                MemoryContent(
                    content=rec.get("target", ""),
                    mime_type=MemoryMimeType.TEXT,
                    metadata={
                        "score":       item.get("score", 1.0),
                        "actor":       rec.get("actor", ""),
                        "action":      rec.get("action", ""),
                        "timestamp":   rec.get("timestamp", ""),
                        "record_type": rec.get("record_type", ""),
                    },
                )
            )
        return MemoryQueryResult(results=memories)

    async def update_context(
        self,
        model_context: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> Any:
        """Inject relevant memories into a model context before LLM call."""
        # Get recent history for this agent
        records = self._client.get_conversation_history(self._agent_id, limit=self._top_k)
        records.sort(key=lambda r: r.get("timestamp", ""))
        if not records:
            if _AUTOGEN_AVAILABLE:
                return UpdateContextResult(memories_added=0)
            return {"memories_added": 0}

        context_text = "\n".join(
            f"[{r.get('action', 'memory')}] {r.get('target', '')}"
            for r in records
        )
        system_msg = {"role": "system", "content": f"[HipCortex memory]\n{context_text}"}

        if hasattr(model_context, "add_message"):
            await model_context.add_message(system_msg)
        elif hasattr(model_context, "messages"):
            model_context.messages.insert(0, system_msg)

        if _AUTOGEN_AVAILABLE:
            return UpdateContextResult(memories_added=len(records))
        return {"memories_added": len(records)}

    async def clear(self) -> None:
        """GDPR right-to-forget: delete all memories for this agent."""
        self._client.forget(self._agent_id)

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def store_observation(self, text: str, action: str = "observation") -> None:
        """Synchronous convenience wrapper — stores a text observation."""
        self._client.add_memory(
            actor=self._agent_id,
            action=action,
            target=text,
            record_type="Temporal",
        )

    def get_context_string(self, limit: int = 20) -> str:
        """Return formatted history as a plain string for non-AutoGen use."""
        records = self._client.get_conversation_history(self._agent_id, limit=limit)
        records.sort(key=lambda r: r.get("timestamp", ""))
        return "\n".join(
            f"[{r.get('action','?')}] {r.get('target','')}" for r in records
        )
