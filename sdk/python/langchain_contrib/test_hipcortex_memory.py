"""Tests for HipCortexChatMessageHistory and HipCortexMemory.

Follows langchain-community test conventions.
Run with: pytest sdk/python/langchain_contrib/test_hipcortex_memory.py -v

Requires a running HipCortex server on localhost:3030:
    cargo run --bin webserver --no-default-features \\
        --features "web-server,petgraph_backend"
"""

from __future__ import annotations

import uuid
from unittest.mock import MagicMock, patch

import pytest

# ---------------------------------------------------------------------------
# Unit tests (no server required — mock the client)
# ---------------------------------------------------------------------------


def _make_history(session_id: str | None = None):
    """Create a HipCortexChatMessageHistory with a mocked client."""
    from langchain_contrib.hipcortex_memory import HipCortexChatMessageHistory

    with patch("hipcortex.HipCortexClient") as MockClient:
        instance = MockClient.return_value
        instance.get_conversation_history.return_value = []
        history = HipCortexChatMessageHistory(
            session_id=session_id or str(uuid.uuid4()),
            url="http://localhost:3030",
        )
        history._client = instance
        return history, instance


def test_add_human_message():
    history, client = _make_history("test-session")
    history.add_user_message("hello")
    client.add_memory.assert_called_once()
    call_kwargs = client.add_memory.call_args[1]
    assert call_kwargs["action"] == "human_message"
    assert call_kwargs["target"] == "hello"
    assert call_kwargs["actor"] == "test-session"


def test_add_ai_message():
    history, client = _make_history("test-session")
    history.add_ai_message("world")
    client.add_memory.assert_called_once()
    call_kwargs = client.add_memory.call_args[1]
    assert call_kwargs["action"] == "ai_message"
    assert call_kwargs["record_type"] == "Reflexion"


def test_messages_sorted_and_typed():
    from langchain_core.messages import AIMessage, HumanMessage

    history, client = _make_history()
    client.get_conversation_history.return_value = [
        {"action": "ai_message",    "target": "Hi!",   "timestamp": "2026-01-01T00:00:01Z"},
        {"action": "human_message", "target": "Hello", "timestamp": "2026-01-01T00:00:00Z"},
    ]
    msgs = history.messages
    assert len(msgs) == 2
    assert isinstance(msgs[0], HumanMessage)
    assert isinstance(msgs[1], AIMessage)
    assert msgs[0].content == "Hello"
    assert msgs[1].content == "Hi!"


def test_clear_calls_forget():
    history, client = _make_history("user-99")
    history.clear()
    client.forget.assert_called_once_with("user-99")


def test_memory_load_variables_string():
    from langchain_contrib.hipcortex_memory import HipCortexMemory

    with patch("hipcortex.HipCortexClient"):
        memory = HipCortexMemory(session_id="s1")
    memory._history._client = MagicMock()
    memory._history._client.get_conversation_history.return_value = [
        {"action": "human_message", "target": "Ping", "timestamp": "2026-01-01T00:00:00Z"},
        {"action": "ai_message",    "target": "Pong", "timestamp": "2026-01-01T00:00:01Z"},
    ]
    result = memory.load_memory_variables({})
    assert "history" in result
    assert "Human: Ping" in result["history"]
    assert "AI: Pong" in result["history"]


def test_memory_save_context():
    from langchain_contrib.hipcortex_memory import HipCortexMemory

    with patch("hipcortex.HipCortexClient"):
        memory = HipCortexMemory(session_id="s2")
    memory._history._client = MagicMock()
    memory.save_context({"input": "Test"}, {"output": "Response"})
    assert memory._history._client.add_memory.call_count == 2


# ---------------------------------------------------------------------------
# Integration tests (require live server — skipped in CI unless HIPCORTEX_URL set)
# ---------------------------------------------------------------------------

import os

HIPCORTEX_URL = os.getenv("HIPCORTEX_URL", "")

@pytest.mark.skipif(not HIPCORTEX_URL, reason="HIPCORTEX_URL not set — skipping integration tests")
def test_integration_round_trip():
    from langchain_contrib.hipcortex_memory import HipCortexChatMessageHistory
    from langchain_core.messages import AIMessage, HumanMessage

    session_id = f"test-{uuid.uuid4().hex[:8]}"
    history = HipCortexChatMessageHistory(session_id=session_id, url=HIPCORTEX_URL)

    history.add_user_message("Integration test message")
    history.add_ai_message("Integration test response")

    msgs = history.messages
    assert len(msgs) == 2
    assert isinstance(msgs[0], HumanMessage)
    assert isinstance(msgs[1], AIMessage)

    history.clear()
    assert history.messages == []
