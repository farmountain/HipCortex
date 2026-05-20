"""Async client tests — run with: pytest sdk/python/tests/test_async_client.py -v"""
import pytest
from unittest.mock import MagicMock, patch

@pytest.mark.asyncio
async def test_async_add_memory():
    from hipcortex.async_client import AsyncHipCortexClient
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {"success": True, "record_id": "abc-123", "error": None}
    mock_response.raise_for_status = MagicMock()

    with patch("httpx.AsyncClient.post", return_value=mock_response) as mock_post:
        client = AsyncHipCortexClient(base_url="http://localhost:3030")
        result = await client.add_memory(actor="alice", action="said", target="hello")
        assert result["success"] is True
        assert result["record_id"] == "abc-123"
        mock_post.assert_called_once()

@pytest.mark.asyncio
async def test_async_search():
    from hipcortex.async_client import AsyncHipCortexClient
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {"results": [{"score": 0.9, "record": {}}], "total": 1}
    mock_response.raise_for_status = MagicMock()

    with patch("httpx.AsyncClient.post", return_value=mock_response):
        client = AsyncHipCortexClient(base_url="http://localhost:3030")
        results = await client.search("hello", limit=5)
        assert len(results) == 1
        assert results[0]["score"] == 0.9

@pytest.mark.asyncio
async def test_async_forget():
    from hipcortex.async_client import AsyncHipCortexClient
    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {"success": True, "actor": "alice", "records_deleted": 3}
    mock_response.raise_for_status = MagicMock()

    with patch("httpx.AsyncClient.request", return_value=mock_response):
        client = AsyncHipCortexClient(base_url="http://localhost:3030")
        result = await client.forget("alice")
        assert result["records_deleted"] == 3

@pytest.mark.asyncio
async def test_async_context_manager():
    from hipcortex.async_client import AsyncHipCortexClient
    async with AsyncHipCortexClient(base_url="http://localhost:3030") as client:
        assert client._client is not None

@pytest.mark.asyncio
async def test_async_health_returns_bool():
    from hipcortex.async_client import AsyncHipCortexClient
    import httpx
    with patch("httpx.AsyncClient.get", side_effect=httpx.RequestError("conn refused")):
        client = AsyncHipCortexClient(base_url="http://localhost:3030")
        ok = await client.health()
        assert ok is False
