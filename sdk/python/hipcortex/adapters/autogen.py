"""HipCortex AutoGen adapter — targets AutoGen 0.4+ (autogen-agentchat >= 0.4).

AutoGen 0.4 changed from register_hook() to the Memory protocol.

Usage (AutoGen 0.4+):
    from autogen_agentchat.agents import AssistantAgent
    from hipcortex.adapters.autogen import HipCortexAutoGenMemory

    memory = HipCortexAutoGenMemory.from_settings()  # config URL + default actor
    agent = AssistantAgent(name="researcher", model_client=..., memory=[memory])

Usage (AutoGen 0.3 — legacy):
    agent.register_hook("process_message_before_send", memory.on_message_sent_v03)
    agent.register_hook("process_all_messages_before_reply", memory.on_messages_received_v03)
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from ..client import HipCortexClient

try:
    from autogen_core.memory import Memory, MemoryContent, MemoryMimeType, MemoryQueryResult
    from autogen_core import CancellationToken
    _AUTOGEN4_AVAILABLE = True
except ImportError:
    _AUTOGEN4_AVAILABLE = False
    class Memory:  # type: ignore[no-redef]
        pass
    class CancellationToken:  # type: ignore[no-redef]
        pass


class HipCortexAutoGenMemory(Memory):  # type: ignore[misc]
    """AutoGen 0.4 Memory protocol backed by HipCortex."""

    def __init__(
        self,
        client: HipCortexClient,
        agent_id: str = "autogen-agent",
        top_k: int = 10,
    ) -> None:
        self._client = client
        self._agent_id = agent_id
        self._top_k = top_k

    @classmethod
    def from_settings(
        cls,
        agent_id: Optional[str] = None,
        top_k: int = 10,
        **kw: Any,
    ) -> "HipCortexAutoGenMemory":
        """Build memory from project/user config (URL + default actor).

        Args:
            agent_id: Actor scope; defaults to :func:`default_session_id`.
            top_k: Max results for query/update_context.
            **kw: Optional ``client`` override and unused extras ignored.
        """
        from .common import client_from_settings, default_session_id

        client = kw.pop("client", None) or client_from_settings()
        if agent_id is None:
            agent_id = default_session_id()
        return cls(client=client, agent_id=agent_id, top_k=top_k)

    # AutoGen 0.4 Memory protocol -------------------------------------------

    async def add(
        self,
        content: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> None:
        if _AUTOGEN4_AVAILABLE and hasattr(content, "content"):
            text = str(content.content)
            role = getattr(content, "role", None)
            mime = getattr(content, "mime_type", None)
            action = "ai_message" if (
                (role and role in ("assistant", "ai")) or
                (mime and "assistant" in str(mime).lower())
            ) else "observation"
        else:
            text = str(content)
            action = "observation"
        self._client.add_memory(
            actor=self._agent_id, action=action,
            target=text, record_type="Temporal",
        )

    async def query(
        self,
        query: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> Any:
        query_text = str(getattr(query, "content", query))
        try:
            search_results = self._client.search(query=query_text, limit=self._top_k)
        except Exception:
            search_results = [
                {"record": r, "score": 1.0}
                for r in self._client.get_conversation_history(self._agent_id, limit=self._top_k)
            ]
        if not _AUTOGEN4_AVAILABLE:
            return search_results
        memories = []
        for item in search_results:
            rec = item.get("record", item)
            memories.append(MemoryContent(
                content=rec.get("target", ""),
                mime_type=MemoryMimeType.TEXT,
                metadata={"score": item.get("score", 1.0), "actor": rec.get("actor", ""),
                          "action": rec.get("action", ""), "timestamp": rec.get("timestamp", "")},
            ))
        return MemoryQueryResult(results=memories)

    async def update_context(
        self,
        model_context: Any,
        cancellation_token: Optional[CancellationToken] = None,
    ) -> Any:
        records = self._client.get_conversation_history(self._agent_id, limit=self._top_k)
        records.sort(key=lambda r: r.get("timestamp", ""))
        if not records:
            return {"memories_added": 0}
        lines = [f"[{r.get('action','memory')}] {r.get('target','')}" for r in records]
        system_msg = {"role": "system", "content": "[HipCortex memory]\n" + "\n".join(lines)}
        if hasattr(model_context, "add_message"):
            await model_context.add_message(system_msg)
        elif hasattr(model_context, "messages"):
            model_context.messages.insert(0, system_msg)
        return {"memories_added": len(records)}

    async def clear(self) -> None:
        self._client.forget(self._agent_id)

    # AutoGen 0.3 legacy hooks -----------------------------------------------

    def on_message_sent_v03(self, message: Dict[str, Any], **kwargs: Any) -> Dict[str, Any]:
        """AutoGen 0.3 hook: process_message_before_send."""
        content = message.get("content", "")
        role = message.get("role", "assistant")
        action = "ai_message" if role == "assistant" else "human_message"
        self._client.add_memory(actor=self._agent_id, action=action, target=str(content),
                                record_type="Reflexion" if role == "assistant" else "Temporal")
        return message

    def on_messages_received_v03(
        self, messages: List[Dict[str, Any]], **kwargs: Any
    ) -> List[Dict[str, Any]]:
        """AutoGen 0.3 hook: process_all_messages_before_reply."""
        history = self._client.get_conversation_history(self._agent_id, limit=20)
        if not history:
            return messages
        history.sort(key=lambda r: r.get("timestamp", ""))
        lines = [
            f"{'AI' if r.get('action') == 'ai_message' else 'Human'}: {r.get('target','')}"
            for r in history
        ]
        return [{"role": "system", "content": "[HipCortex memory]\n" + "\n".join(lines)}] + list(messages)

    # Sync helpers (no AutoGen needed) ---------------------------------------

    def store(self, content: str, action: str = "observation") -> Dict[str, Any]:
        return self._client.add_memory(actor=self._agent_id, action=action,
                                       target=content, record_type="Temporal")

    def recall(self, limit: int = 20) -> str:
        records = self._client.get_conversation_history(self._agent_id, limit=limit)
        records.sort(key=lambda r: r.get("timestamp", ""))
        return "\n".join(f"[{r.get('action','?')}] {r.get('target','')}" for r in records)

    def forget_all(self) -> Dict[str, Any]:
        return self._client.forget(self._agent_id)


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
        self._capture("function_call", f"{name}({str(arguments)[:80]})")

    def on_function_result(self, name: str, result: str) -> None:
        self._capture("function_result", f"{name} → {str(result)[:80]}")

    def make_v03_send_hook(self):
        """Hook for AutoGen v0.3 send hook."""
        def _hook(message: dict) -> None:
            try:
                content = message.get("content", "") or ""
                role = message.get("role", "unknown")
                self._capture("message_sent", f"[{role}] {str(content)[:100]}")
            except Exception:
                pass
        return _hook

    def make_v03_receive_hook(self):
        """Hook for AutoGen v0.3 receive hook."""
        def _hook(message: dict, sender=None) -> None:
            try:
                content = message.get("content", "") or ""
                sender_name = getattr(sender, "name", str(sender)) if sender else "unknown"
                self._capture("message_received", f"[{sender_name}] {str(content)[:100]}")
            except Exception:
                pass
        return _hook
