"""Tests for AsyncHipCortexMemory — run: pytest sdk/python/tests/test_async_langchain.py -v"""
import pytest
from unittest.mock import AsyncMock, MagicMock


@pytest.mark.asyncio
async def test_async_load_memory_variables_empty():
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient
    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.get_conversation_history = AsyncMock(return_value=[])
    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-1")
    result = await memory.aload_memory_variables({})
    assert result == {"history": ""}
    mock_client.get_conversation_history.assert_awaited_once_with("sess-1", limit=50)


@pytest.mark.asyncio
async def test_async_load_memory_variables_with_records():
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient
    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.get_conversation_history = AsyncMock(return_value=[
        {"action": "human_message", "target": "Hello", "timestamp": "2026-01-01T00:00:00Z"},
        {"action": "ai_message",    "target": "Hi!",   "timestamp": "2026-01-01T00:00:01Z"},
    ])
    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-2")
    result = await memory.aload_memory_variables({})
    assert result["history"] == "Human: Hello\nAI: Hi!"


@pytest.mark.asyncio
async def test_async_save_context():
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient
    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.add_human_message = AsyncMock(return_value={"success": True})
    mock_client.add_ai_message    = AsyncMock(return_value={"success": True})
    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-3")
    await memory.asave_context({"input": "How are you?"}, {"output": "I am fine."})
    mock_client.add_human_message.assert_awaited_once_with("sess-3", "How are you?")
    mock_client.add_ai_message.assert_awaited_once_with("sess-3", "I am fine.")


@pytest.mark.asyncio
async def test_async_clear():
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient
    mock_client = AsyncMock(spec=AsyncHipCortexClient)
    mock_client.forget = AsyncMock(return_value={"success": True})
    memory = AsyncHipCortexMemory(client=mock_client, session_id="sess-4")
    await memory.aclear()
    mock_client.forget.assert_awaited_once_with("sess-4")


def test_memory_variables_property():
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    from hipcortex.async_client import AsyncHipCortexClient
    mock_client = MagicMock(spec=AsyncHipCortexClient)
    memory = AsyncHipCortexMemory(client=mock_client, session_id="x", memory_key="chat_history")
    assert memory.memory_variables == ["chat_history"]
