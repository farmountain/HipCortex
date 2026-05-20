"""langchain_community.memory.HipCortexChatMessageHistory
and langchain_community.memory.HipCortexMemory

This file is formatted for upstream contribution to
langchain-ai/langchain-community.

Target path:
  langchain_community/memory/hipcortex.py

PR description:
  Adds HipCortex (https://github.com/farmountain/HipCortex) as a
  persistent chat memory backend for LangChain.
  HipCortex stores messages with temporal decay, causal world-modeling,
  and a Merkle-chained audit log — fully self-hostable with zero external deps.

Dependencies:
  pip install hipcortex  (wraps HipCortex REST API via requests)

Compatibility:
  langchain-core >= 0.1.0
  langchain-community >= 0.0.1
  Python >= 3.9
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Sequence

from langchain_core.chat_history import BaseChatMessageHistory
from langchain_core.messages import (
    AIMessage,
    BaseMessage,
    HumanMessage,
    SystemMessage,
    messages_from_dict,
    messages_to_dict,
)
from langchain_core.memory import BaseMemory


class HipCortexChatMessageHistory(BaseChatMessageHistory):
    """Chat message history backed by a HipCortex memory server.

    HipCortex stores messages as typed MemoryRecords with temporal decay,
    causal graph linking, and a Merkle-chained audit log. It is fully
    self-hostable (single Rust binary, zero external dependencies) and
    exposes a REST API consumed here via the ``hipcortex`` Python SDK.

    Install:
        pip install hipcortex

    Quickstart:
        history = HipCortexChatMessageHistory(
            session_id="user-42",
            url="http://localhost:3030",
        )
        history.add_user_message("Hi!")
        history.add_ai_message("Hello!")
        print(history.messages)

    Args:
        session_id: Scopes all messages to this session / user identifier.
        url:        Base URL of the HipCortex server (default: localhost:3030).
        api_key:    Optional API key for ``X-Api-Key`` header (managed SaaS).
        max_limit:  Max messages returned by ``messages`` property (default 100).
    """

    def __init__(
        self,
        session_id: str,
        url: str = "http://localhost:3030",
        api_key: Optional[str] = None,
        max_limit: int = 100,
    ) -> None:
        try:
            from hipcortex import HipCortexClient  # type: ignore[import]
        except ImportError as exc:
            raise ImportError(
                "Could not import hipcortex. Install with: pip install hipcortex"
            ) from exc

        self.session_id = session_id
        self.max_limit = max_limit
        import requests

        session = requests.Session()
        if api_key:
            session.headers["X-Api-Key"] = api_key
        self._client = HipCortexClient(base_url=url)
        self._client._session = session

    # ------------------------------------------------------------------
    # BaseChatMessageHistory interface
    # ------------------------------------------------------------------

    @property
    def messages(self) -> List[BaseMessage]:  # type: ignore[override]
        """Retrieve all messages for this session, oldest-first."""
        records = self._client.get_conversation_history(
            self.session_id, limit=self.max_limit
        )
        records.sort(key=lambda r: r.get("timestamp", ""))
        result: List[BaseMessage] = []
        for rec in records:
            action = rec.get("action", "")
            content = rec.get("target", "")
            if action == "human_message":
                result.append(HumanMessage(content=content))
            elif action == "ai_message":
                result.append(AIMessage(content=content))
            elif action == "system_message":
                result.append(SystemMessage(content=content))
        return result

    def add_message(self, message: BaseMessage) -> None:
        """Persist a single message."""
        if isinstance(message, HumanMessage):
            action, record_type = "human_message", "Temporal"
        elif isinstance(message, AIMessage):
            action, record_type = "ai_message", "Reflexion"
        elif isinstance(message, SystemMessage):
            action, record_type = "system_message", "Symbolic"
        else:
            action, record_type = "message", "Temporal"
        self._client.add_memory(
            actor=self.session_id,
            action=action,
            target=message.content,
            record_type=record_type,
        )

    def clear(self) -> None:
        """Delete all messages for this session (GDPR right-to-forget)."""
        self._client.forget(self.session_id)


class HipCortexMemory(BaseMemory):
    """LangChain ``BaseMemory`` backed by HipCortex persistent memory server.

    Drop-in replacement for ``ConversationBufferMemory`` with the added benefit
    of temporal decay, causal world-modeling, and a tamper-evident audit log.

    Quickstart:
        from langchain.chains import ConversationChain
        from langchain.chat_models import ChatOpenAI
        from langchain_community.memory import HipCortexMemory

        memory = HipCortexMemory(session_id="user-42", url="http://localhost:3030")
        chain  = ConversationChain(llm=ChatOpenAI(), memory=memory)
        chain.predict(input="What do you remember?")

    Args:
        session_id:   Scopes memory to this user / conversation identifier.
        url:          HipCortex server base URL.
        api_key:      Optional ``X-Api-Key`` for managed SaaS tier.
        memory_key:   Key injected into the chain's prompt variables.
        human_prefix: Label for human turns in formatted history.
        ai_prefix:    Label for AI turns in formatted history.
        return_messages: If True, return list[BaseMessage]; else formatted string.
        max_limit:    Max records fetched per call.
    """

    # Avoid Pydantic model dependency for cross-version compatibility
    def __init__(
        self,
        session_id: str = "default",
        url: str = "http://localhost:3030",
        api_key: Optional[str] = None,
        memory_key: str = "history",
        human_prefix: str = "Human",
        ai_prefix: str = "AI",
        return_messages: bool = False,
        max_limit: int = 100,
    ) -> None:
        self._history = HipCortexChatMessageHistory(
            session_id=session_id,
            url=url,
            api_key=api_key,
            max_limit=max_limit,
        )
        self.memory_key = memory_key
        self.human_prefix = human_prefix
        self.ai_prefix = ai_prefix
        self.return_messages = return_messages

    @property
    def memory_variables(self) -> List[str]:
        return [self.memory_key]

    def load_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        messages = self._history.messages
        if self.return_messages:
            return {self.memory_key: messages}
        lines: List[str] = []
        for msg in messages:
            if isinstance(msg, HumanMessage):
                lines.append(f"{self.human_prefix}: {msg.content}")
            elif isinstance(msg, AIMessage):
                lines.append(f"{self.ai_prefix}: {msg.content}")
        return {self.memory_key: "\n".join(lines)}

    def save_context(self, inputs: Dict[str, Any], outputs: Dict[str, Any]) -> None:
        human_text = inputs.get("input") or inputs.get("human_input") or ""
        ai_text    = outputs.get("output") or outputs.get("response") or ""
        if human_text:
            self._history.add_message(HumanMessage(content=str(human_text)))
        if ai_text:
            self._history.add_message(AIMessage(content=str(ai_text)))

    def clear(self) -> None:
        self._history.clear()
