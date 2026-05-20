"""LlamaIndex storage context wrapper backed by HipCortex.

Provides a ``HipCortexStorageContext`` that can be passed wherever LlamaIndex
accepts a ``StorageContext``, and a ``HipCortexChatStore`` compatible with
``SimpleChatStore`` for chat-engine memory.

Usage::

    from hipcortex import HipCortexClient
    from hipcortex.llamaindex_storage import HipCortexChatStore
    from llama_index.core.memory import ChatMemoryBuffer

    client = HipCortexClient(base_url="http://localhost:3000")
    store = HipCortexChatStore(client=client)

    memory = ChatMemoryBuffer.from_defaults(
        token_limit=3000,
        chat_store=store,
        chat_store_key="user-42",
    )
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Sequence

from .client import HipCortexClient


class HipCortexChatStore:
    """Drop-in for ``SimpleChatStore`` in LlamaIndex chat engines.

    Stores each chat message as a HipCortex ``MemoryRecord`` keyed by
    ``session_key`` (the ``chat_store_key`` param in LlamaIndex).
    """

    def __init__(self, client: HipCortexClient) -> None:
        self.client = client

    # LlamaIndex SimpleChatStore interface ----------------------------------

    def set_messages(self, key: str, messages: Sequence[Any]) -> None:
        """Overwrite messages for ``key`` by forgetting then re-adding."""
        self.client.forget(key)
        for msg in messages:
            role = getattr(msg, "role", "user")
            content = getattr(msg, "content", str(msg))
            action = "ai_message" if role in ("assistant", "ai") else "human_message"
            self.client.add_memory(
                actor=key,
                action=action,
                target=content,
                record_type="Temporal" if role != "assistant" else "Reflexion",
            )

    def get_messages(self, key: str) -> List[Any]:
        """Return raw record dicts (caller converts to ChatMessage objects)."""
        records = self.client.get_conversation_history(key, limit=200)
        records.sort(key=lambda r: r.get("timestamp", ""))
        return records

    def add_message(self, key: str, message: Any) -> None:
        role = getattr(message, "role", "user")
        content = getattr(message, "content", str(message))
        action = "ai_message" if role in ("assistant", "ai") else "human_message"
        self.client.add_memory(
            actor=key,
            action=action,
            target=content,
            record_type="Temporal" if role != "assistant" else "Reflexion",
        )

    def delete_messages(self, key: str) -> Optional[List[Any]]:
        msgs = self.get_messages(key)
        self.client.forget(key)
        return msgs

    def delete_message(self, key: str, idx: int) -> Optional[Any]:
        """Not efficiently supported; falls back to forget-and-reinsert."""
        msgs = self.get_messages(key)
        if idx < 0 or idx >= len(msgs):
            return None
        removed = msgs.pop(idx)
        self.set_messages(key, msgs)  # type: ignore[arg-type]
        return removed

    def get_keys(self) -> List[str]:
        """Returns empty list — full key enumeration requires graph query."""
        return []


class HipCortexStorageContext:
    """Lightweight wrapper that holds a ``HipCortexChatStore`` and exposes
    a ``StorageContext``-like surface for LlamaIndex.

    For simple use-cases (chat engines, memory buffers) this is sufficient.
    For full index persistence (VectorStore, DocStore, IndexStore) use
    LlamaIndex's native backends alongside HipCortex for chat memory only.
    """

    def __init__(self, client: HipCortexClient) -> None:
        self.client = client
        self.chat_store = HipCortexChatStore(client=client)

    @classmethod
    def from_url(cls, base_url: str = "http://localhost:3000") -> "HipCortexStorageContext":
        return cls(client=HipCortexClient(base_url=base_url))
