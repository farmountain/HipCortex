"""HipCortex async HTTP client — thin async wrapper around the REST API using httpx."""

from __future__ import annotations

import time
from typing import Any, Dict, List, Optional

import httpx


class AsyncHipCortexClient:
    """Asynchronous HTTP client for the HipCortex memory server.

    Mirrors :class:`HipCortexClient` with coroutine-based methods powered by
    ``httpx.AsyncClient``.  Compatible with FastAPI, Django async views,
    LangChain 0.3+ async chains, and any asyncio / anyio event loop.

    Args:
        base_url: Root URL of the running hipcortex web-server binary.
        timeout:  Per-request timeout in seconds.
        api_key:  Optional Bearer token sent as ``Authorization: Bearer <key>``.

    Usage::

        # One-shot (opens and closes the underlying httpx.AsyncClient per call)
        client = AsyncHipCortexClient()
        result = await client.add_memory("alice", "said", "hello")

        # Context-manager (recommended for sustained use)
        async with AsyncHipCortexClient() as client:
            result = await client.add_memory("alice", "said", "hello")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:3030",
        timeout: float = 10.0,
        api_key: Optional[str] = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self._api_key = api_key
        headers: Dict[str, str] = {}
        if api_key:
            headers["Authorization"] = f"Bearer {api_key}"
        self._client = httpx.AsyncClient(headers=headers, timeout=timeout)

    # ------------------------------------------------------------------
    # Async context-manager support
    # ------------------------------------------------------------------

    async def __aenter__(self) -> "AsyncHipCortexClient":
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.close()

    async def close(self) -> None:
        """Close the underlying :class:`httpx.AsyncClient`."""
        await self._client.aclose()

    # ------------------------------------------------------------------
    # Core memory operations
    # ------------------------------------------------------------------

    async def add_memory(
        self,
        actor: str,
        action: str,
        target: str,
        record_type: str = "Temporal",
        metadata: Optional[Dict[str, Any]] = None,
        ttl_seconds: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Store a memory record.

        Returns the server response dict with ``success`` and ``record_id``.
        """
        payload: Dict[str, Any] = {
            "actor": actor,
            "action": action,
            "target": target,
            "record_type": record_type,
            "metadata": metadata or {},
        }
        if ttl_seconds is not None:
            payload["ttl_seconds"] = ttl_seconds
        resp = await self._client.post(f"{self.base_url}/memory/add", json=payload)
        resp.raise_for_status()
        return resp.json()

    async def query_memory(
        self,
        actor: Optional[str] = None,
        action: Optional[str] = None,
        record_type: Optional[str] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        """Query memory records. Returns a list of record dicts."""
        params: Dict[str, Any] = {"limit": limit}
        if actor is not None:
            params["actor"] = actor
        if action is not None:
            params["action"] = action
        if record_type is not None:
            params["record_type"] = record_type
        resp = await self._client.get(f"{self.base_url}/memory/query", params=params)
        resp.raise_for_status()
        return resp.json().get("records", [])

    async def search(
        self,
        query: str,
        embedding: Optional[List[float]] = None,
        limit: int = 10,
    ) -> List[Dict[str, Any]]:
        """Semantic + keyword search over stored memory records.

        If ``embedding`` is provided, ranks results by cosine similarity
        against records that carry a ``metadata.embedding`` float array.
        Falls back to keyword matching when embeddings are absent.

        Returns a list of ``{"score": float, "record": {...}}`` dicts,
        sorted by descending score.
        """
        payload: Dict[str, Any] = {"query": query, "limit": limit}
        if embedding is not None:
            payload["embedding"] = embedding
        resp = await self._client.post(f"{self.base_url}/memory/search", json=payload)
        resp.raise_for_status()
        return resp.json().get("results", [])

    async def bulk_add(self, records: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Bulk-insert multiple memory records in a single request.

        Args:
            records: List of record dicts, each with at minimum
                     ``actor``, ``action``, and ``target`` keys.

        Returns the server response with ``inserted`` count and any errors.
        """
        resp = await self._client.post(
            f"{self.base_url}/memory/bulk", json={"records": records}
        )
        resp.raise_for_status()
        return resp.json()

    async def embed_and_add(
        self,
        actor: str,
        action: str,
        target: str,
        embedding_model: str,
        record_type: str = "Temporal",
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """Store a memory record and generate an embedding server-side.

        The server calls the specified ``embedding_model`` (e.g. an Ollama
        or OpenAI model) and attaches the resulting vector to the record.

        Returns the server response dict with ``success`` and ``record_id``.
        """
        payload: Dict[str, Any] = {
            "actor": actor,
            "action": action,
            "target": target,
            "record_type": record_type,
            "embedding_model": embedding_model,
            "metadata": metadata or {},
        }
        resp = await self._client.post(f"{self.base_url}/memory/embed", json=payload)
        resp.raise_for_status()
        return resp.json()

    async def forget(self, actor: str) -> Dict[str, Any]:
        """GDPR right-to-forget: delete all records for ``actor``.

        Returns ``{"success": bool, "records_deleted": int, "symbolic_nodes_deleted": int}``.
        """
        resp = await self._client.delete(
            f"{self.base_url}/memory/forget/{actor}"
        )
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # System
    # ------------------------------------------------------------------

    async def health(self) -> bool:
        """Return True if the server is reachable and healthy."""
        try:
            resp = await self._client.get(f"{self.base_url}/health")
            return resp.status_code == 200
        except httpx.RequestError:
            return False

    async def stats(self) -> Dict[str, Any]:
        """Return live server statistics: record counts, type breakdown, metering state."""
        resp = await self._client.get(f"{self.base_url}/stats")
        resp.raise_for_status()
        return resp.json()

    async def coherence_status(self) -> Dict[str, Any]:
        """Return the current coherence metrics from the server."""
        resp = await self._client.get(f"{self.base_url}/coherence/status")
        resp.raise_for_status()
        return resp.json()

    async def graph(self) -> Dict[str, Any]:
        """Return the full symbolic graph (nodes + edges)."""
        resp = await self._client.get(f"{self.base_url}/graph")
        resp.raise_for_status()
        return resp.json()

    async def get_node(self, node_id: str) -> Optional[Dict[str, Any]]:
        """Fetch a single symbolic node by UUID."""
        resp = await self._client.get(f"{self.base_url}/node/{node_id}")
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # Convenience helpers used by framework adapters
    # ------------------------------------------------------------------

    async def add_human_message(self, session_id: str, content: str) -> Dict[str, Any]:
        """Record a human turn in a conversation session."""
        return await self.add_memory(
            actor=session_id,
            action="human_message",
            target=content,
            record_type="Temporal",
        )

    async def add_ai_message(self, session_id: str, content: str) -> Dict[str, Any]:
        """Record an AI turn in a conversation session."""
        return await self.add_memory(
            actor=session_id,
            action="ai_message",
            target=content,
            record_type="Reflexion",
        )

    async def get_conversation_history(
        self, session_id: str, limit: int = 50
    ) -> List[Dict[str, Any]]:
        """Retrieve the message history for a conversation session."""
        return await self.query_memory(actor=session_id, limit=limit)

    async def ping_latency_ms(self) -> float:
        """Return round-trip latency to /health in milliseconds."""
        t0 = time.perf_counter()
        await self.health()
        return (time.perf_counter() - t0) * 1000
