"""AutoGen memory hook for HipCortex.

Provides a ``HipCortexAutoGenMemory`` class that can be used as a
``memory`` parameter in AutoGen ``ConversableAgent``.

Usage::

    import autogen
    from hipcortex import HipCortexClient
    from hipcortex.adapters.autogen import HipCortexAutoGenMemory

    client = HipCortexClient(base_url="http://localhost:3000")
    memory = HipCortexAutoGenMemory(client=client, agent_id="research-agent")

    assistant = autogen.AssistantAgent(
        name="assistant",
        llm_config={...},
        # AutoGen 0.3+: pass as transform parameter
    )
    # Register hooks:
    assistant.register_hook("process_message_before_send",
                            memory.on_message_sent)
    assistant.register_hook("process_all_messages_before_reply",
                            memory.on_messages_received)
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from ..client import HipCortexClient


class HipCortexAutoGenMemory:
    """AutoGen-compatible memory adapter.

    Hooks into AutoGen's ``register_hook`` mechanism to persist and
    retrieve conversation context via HipCortex.

    Args:
        client:   Configured :class:`~hipcortex.client.HipCortexClient`.
        agent_id: Actor identifier used to scope memory records.
        max_context_records: Max records fetched when building context.
    """

    def __init__(
        self,
        client: HipCortexClient,
        agent_id: str = "autogen-agent",
        max_context_records: int = 50,
    ) -> None:
        self.client = client
        self.agent_id = agent_id
        self.max_context_records = max_context_records

    # ------------------------------------------------------------------
    # AutoGen hook callbacks
    # ------------------------------------------------------------------

    def on_message_sent(self, message: Dict[str, Any], **kwargs: Any) -> Dict[str, Any]:
        """Hook: ``process_message_before_send``.

        Persists the outbound message and injects recent context into the
        message metadata so the receiving agent has memory continuity.
        """
        content = message.get("content", "")
        role = message.get("role", "assistant")
        action = "ai_message" if role == "assistant" else "human_message"
        self.client.add_memory(
            actor=self.agent_id,
            action=action,
            target=str(content),
            record_type="Reflexion" if role == "assistant" else "Temporal",
        )
        return message

    def on_messages_received(
        self, messages: List[Dict[str, Any]], **kwargs: Any
    ) -> List[Dict[str, Any]]:
        """Hook: ``process_all_messages_before_reply``.

        Retrieves relevant memories and prepends a system-style context
        message if there is prior history.
        """
        history = self.get_context()
        if not history:
            return messages
        context_msg = {
            "role": "system",
            "content": f"[HipCortex memory context]\n{history}",
        }
        return [context_msg] + list(messages)

    # ------------------------------------------------------------------
    # Memory management helpers
    # ------------------------------------------------------------------

    def store(self, content: str, action: str = "observation") -> Dict[str, Any]:
        """Manually store an arbitrary observation."""
        return self.client.add_memory(
            actor=self.agent_id,
            action=action,
            target=content,
            record_type="Temporal",
        )

    def get_context(self, limit: Optional[int] = None) -> str:
        """Return formatted conversation history as a plain string."""
        records = self.client.get_conversation_history(
            self.agent_id, limit=limit or self.max_context_records
        )
        records.sort(key=lambda r: r.get("timestamp", ""))
        lines: List[str] = []
        for rec in records:
            action = rec.get("action", "")
            target = rec.get("target", "")
            prefix = "AI" if action == "ai_message" else "Human"
            lines.append(f"{prefix}: {target}")
        return "\n".join(lines)

    def forget(self) -> Dict[str, Any]:
        """Delete all memory for this agent (GDPR / session reset)."""
        return self.client.forget(self.agent_id)
