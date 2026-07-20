"""HipCortex HTTP client — thin wrapper around the REST API."""

from __future__ import annotations

import json
import time
from typing import Any, Dict, List, Optional

import requests


class HipCortexClient:
    """Synchronous HTTP client for the HipCortex memory server.

    Args:
        base_url: Root URL. When ``None``, resolved via
            :func:`hipcortex.config.load_settings` (``HIPCORTEX_URL`` >
            project ``.hipcortex/config.toml`` > user config > default).
        timeout:  Per-request timeout in seconds.
    """

    def __init__(self, base_url: str | None = None, timeout: float = 10.0) -> None:
        if base_url is None:
            from .config import load_settings

            base_url = load_settings().url
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self._session = requests.Session()

    # ------------------------------------------------------------------
    # Core memory operations
    # ------------------------------------------------------------------

    def add_memory(
        self,
        actor: str,
        action: str,
        target: str,
        record_type: str = "Temporal",
        metadata: Optional[Dict[str, Any]] = None,
        tags: Optional[List[str]] = None,
        priority: str = "normal",  # "pinned"|"high"|"normal"|"low"
    ) -> Dict[str, Any]:
        """Store a memory record.

        Returns the server response dict with ``success`` and ``record_id``.
        """
        payload = {
            "actor": actor,
            "action": action,
            "target": target,
            "record_type": record_type,
            "metadata": metadata or {},
        }
        if tags:
            payload["tags"] = tags
        if priority != "normal":
            payload["priority"] = priority
        resp = self._session.post(
            f"{self.base_url}/memory/add", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json()

    def query_memory(
        self,
        actor: Optional[str] = None,
        action: Optional[str] = None,
        record_type: Optional[str] = None,
        limit: int = 100,
        tags: Optional[List[str]] = None,    # filter by tags (any match)
        as_of: Optional[str] = None,         # ISO 8601 timestamp for time-travel query
        include_expired: bool = False,       # include TTL-expired records
    ) -> List[Dict[str, Any]]:
        """Query memory records. Returns a list of record dicts."""
        params: Dict[str, Any] = {"limit": limit}
        if actor is not None:
            params["actor"] = actor
        if action is not None:
            params["action"] = action
        if record_type is not None:
            params["record_type"] = record_type
        if tags is not None:
            params["tags"] = ",".join(tags)
        if as_of is not None:
            params["as_of"] = as_of
        if include_expired:
            params["include_expired"] = "true"
        resp = self._session.get(
            f"{self.base_url}/memory/query", params=params, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json().get("records", [])

    def search(
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
        resp = self._session.post(
            f"{self.base_url}/memory/search", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json().get("results", [])

    def forget(self, actor: str) -> Dict[str, Any]:
        """GDPR right-to-forget: delete all records for ``actor``.

        Returns ``{"success": bool, "records_deleted": int, "symbolic_nodes_deleted": int}``.
        """
        resp = self._session.delete(
            f"{self.base_url}/memory/forget/{actor}", timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json()

    def link_memories(
        self,
        source_id: str,
        target_id: str,
        relation: str = "related",
    ) -> Dict[str, Any]:
        """Create a directed graph edge between two memory records.

        Args:
            source_id: UUID of the source memory record.
            target_id: UUID of the target memory record.
            relation:  Edge label (default: "related").

        Returns:
            ``{"success": bool, "source_id": str, "target_id": str, "relation": str}``
        """
        resp = self._session.post(
            f"{self.base_url}/memory/link",
            json={"source_id": source_id, "target_id": target_id, "relation": relation},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def get_neighbors(self, record_id: str, limit: int = 10) -> List[Dict[str, Any]]:
        """Return neighboring memory records linked via the CausalTopoGraph.

        Args:
            record_id: UUID of the seed memory record.
            limit:     Maximum number of neighbors to return.

        Returns:
            List of MemoryRecord dicts.
        """
        resp = self._session.get(
            f"{self.base_url}/memory/neighbors/{record_id}",
            params={"limit": limit},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json().get("records", [])

    def search_related(self, seed_id: str, limit: int = 10) -> List[Dict[str, Any]]:
        """PPR-ranked related memory search seeded from a given record ID.

        Uses Personalized PageRank (α=0.85, 20 power-iteration rounds) over the
        CausalTopoGraph to surface the most contextually related memories.

        Args:
            seed_id: UUID of the seed memory record to personalise from.
            limit:   Maximum number of related results to return.

        Returns:
            List of ``{"record": {...}, "score": float}`` dicts, sorted by descending score.
        """
        resp = self._session.get(
            f"{self.base_url}/memory/search/related",
            params={"seed_id": seed_id, "limit": limit},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json().get("results", [])

    def delete_memory(self, record_id: str) -> Dict[str, Any]:
        """Delete a single memory record by UUID.

        Args:
            record_id: UUID of the record to delete.

        Returns:
            ``{"success": bool, "record_id": str}``
        """
        resp = self._session.delete(
            f"{self.base_url}/memory/{record_id}", timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # Symbolic graph
    # ------------------------------------------------------------------

    def graph(self) -> Dict[str, Any]:
        """Return the full symbolic graph (nodes + edges)."""

        resp = self._session.get(f"{self.base_url}/graph", timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    def get_node(self, node_id: str) -> Optional[Dict[str, Any]]:
        """Fetch a single symbolic node by UUID."""
        resp = self._session.get(f"{self.base_url}/node/{node_id}", timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # System
    # ------------------------------------------------------------------

    def health(self) -> bool:
        """Return True if server is reachable and healthy."""
        try:
            resp = self._session.get(f"{self.base_url}/health", timeout=self.timeout)
            return resp.status_code == 200
        except requests.RequestException:
            return False

    def coherence_status(self) -> Dict[str, Any]:
        """Return the current coherence metrics from the server."""
        resp = self._session.get(f"{self.base_url}/coherence/status", timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    def live_beliefs(self) -> Dict[str, Any]:
        """Fetch the unified live beliefs (symbolic facts, hypotheses, world predictions)."""
        resp = self._session.get(f"{self.base_url}/memory/live_beliefs", timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    def reflect(self, query: str) -> Dict[str, Any]:
        """POST /memory/reflect — AureusBridge CoT / hypothesis sample over memory context.

        Args:
            query: What to reflect on (e.g. architectural choice, counterfactual).

        Returns:
            Dict with ``hypothesis``, ``confidence``, ``evidence``, ``loops_run``,
            ``llm_available``, ``is_fallback`` (or ``error`` on failure).
        """
        resp = self._session.post(
            f"{self.base_url}/memory/reflect",
            json={"query": query},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def predict(self, state: str, action: str) -> Dict[str, Any]:
        """POST /worldmodel/predict — single-step P(s'|s,a) world-model prediction.

        Args:
            state:  Current world-model state label.
            action: Action applied from that state.

        Returns:
            Dict with ``from_state``, ``action``, ``probabilities``, ``entropy``,
            ``observation_count`` (or ``error`` on failure).
        """
        resp = self._session.post(
            f"{self.base_url}/worldmodel/predict",
            json={"state": state, "action": action},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def stats(self) -> Dict[str, Any]:
        """Return live server statistics: record counts, type breakdown, metering state."""
        resp = self._session.get(f"{self.base_url}/stats", timeout=self.timeout)
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # Convenience helpers used by framework adapters
    # ------------------------------------------------------------------

    def add_human_message(self, session_id: str, content: str) -> Dict[str, Any]:
        return self.add_memory(
            actor=session_id,
            action="human_message",
            target=content,
            record_type="Temporal",
        )

    def add_ai_message(self, session_id: str, content: str) -> Dict[str, Any]:
        return self.add_memory(
            actor=session_id,
            action="ai_message",
            target=content,
            record_type="Reflexion",
        )

    def get_conversation_history(
        self, session_id: str, limit: int = 50
    ) -> List[Dict[str, Any]]:
        return self.query_memory(actor=session_id, limit=limit)

    def create_node(self, label: str, properties: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        """Create a symbolic knowledge graph node."""
        resp = self._session.post(
            f"{self.base_url}/graph/node",
            json={"label": label, "properties": properties or {}},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def create_edge(self, from_id: str, to_id: str, relation: str) -> Dict[str, Any]:
        """Create a relationship edge in the knowledge graph."""
        resp = self._session.post(
            f"{self.base_url}/graph/edge",
            json={"from_id": from_id, "to_id": to_id, "relation": relation},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def set_state(
        self,
        actor: str,
        key: str,
        value: str,
        confidence: float = 1.0,
    ) -> Dict[str, Any]:
        """Store a structured state value for an actor.

        Structured state is permanent (no TTL), high-confidence, pinned.
        Use for: current goal, chosen stack, constraints, key decisions.

        Args:
            actor: Scope identifier (e.g. project name)
            key:   State key (e.g. "goal", "stack", "constraint:no-redis")
            value: State value (free text or JSON string)
            confidence: How confident (default 1.0)

        Returns: stored record info
        """
        return self.add_memory(
            actor=actor,
            action=f"state:{key}",
            target=value,
            record_type="Symbolic",
            priority="pinned",
            metadata={"confidence": str(confidence), "source": "user-state"},
        )

    def get_state(
        self,
        actor: str,
        key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Retrieve structured state for an actor.

        Args:
            actor: Scope identifier
            key:   Optional specific key (e.g. "goal"). If None, returns all state.

        Returns: dict of {key: value} or latest value string if key specified
        """
        action_filter = f"state:{key}" if key else None
        records = self.query_memory(actor=actor, action=action_filter, limit=50)

        if key:
            # Return latest value for this specific key
            if records:
                return {"key": key, "value": records[-1].get("target", ""), "actor": actor}
            return {"key": key, "value": None, "actor": actor}

        # Return all state as dict
        state: Dict[str, Any] = {}
        for rec in records:
            action = rec.get("action", "")
            if action.startswith("state:"):
                k = action[len("state:"):]
                state[k] = rec.get("target", "")
        return state

    def consolidate(self, actor: Optional[str] = None, threshold: float = 0.8, dry_run: bool = True) -> Dict[str, Any]:
        """Find near-duplicate memories and optionally merge them."""
        params: Dict[str, Any] = {"threshold": threshold, "dry_run": dry_run}
        if actor is not None:
            params["actor"] = actor
        resp = self._session.post(
            f"{self.base_url}/memory/consolidate",
            params=params,
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # Zero-config convenience API (Sim #9)
    # ------------------------------------------------------------------

    def remember(
        self,
        text: str,
        actor: Optional[str] = None,
        session_id: Optional[str] = None,
        context: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Zero-config memory ingest — auto-classifies everything from plain text.

        No memory architecture required. HipCortex auto-detects:
        - record_type (Symbolic/Temporal/Reflexion/Procedural)
        - priority (pinned/high/normal/low)
        - TTL (working vs long-term memory)
        - tags, actor, action, confidence

        Args:
            text:       Plain text to remember.
            actor:      Optional actor override (auto-extracted from text if absent).
            session_id: Optional session grouping.
            context:    Hint: "meeting" | "code" | "chat" | "sensor" | "decision"

        Returns:
            Classification result dict with record_id, record_type, priority, tags, etc.
        """
        payload: Dict[str, Any] = {"text": text}
        if actor is not None:
            payload["actor"] = actor
        if session_id is not None:
            payload["session_id"] = session_id
        if context is not None:
            payload["context"] = context
        resp = self._session.post(
            f"{self.base_url}/memory/ingest", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json()

    def recall(
        self,
        query: str,
        actor: Optional[str] = None,
        limit: int = 10,
    ) -> List[str]:
        """Zero-config memory recall — returns plain text memories matching query.

        Returns list of strings (not raw records) for easy use in prompts.

        Args:
            query: What to search for.
            actor: Optional actor filter.
            limit: Max results.

        Returns:
            List of memory strings like "[action] target text"
        """
        params: Dict[str, Any] = {"query": query, "limit": limit}
        if actor is not None:
            params["actor"] = actor
        resp = self._session.get(
            f"{self.base_url}/memory/search-flat", params=params, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json().get("memories", [])

    def recall_with_metadata(
        self,
        query: str,
        actor: Optional[str] = None,
        limit: int = 10,
        min_confidence: Optional[float] = None,
    ) -> List[Dict[str, Any]]:
        """Recall memories with full metadata (confidence, source, tags, priority, version).

        Use when you need to inspect record quality, filter by confidence, or pass
        source attribution to downstream logic.

        Args:
            query:          What to search for.
            actor:          Optional actor filter (applied client-side).
            limit:          Max results from server.
            min_confidence: If set, drop records with confidence < this value.

        Returns:
            List of {"score": float, "record": {id, actor, action, target,
            confidence, source, tags, priority, version, status, ...}} dicts.
        """
        payload: Dict[str, Any] = {"query": query, "limit": limit}
        resp = self._session.post(
            f"{self.base_url}/memory/search", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        results: List[Dict[str, Any]] = resp.json().get("results", [])
        if actor is not None:
            results = [r for r in results if r.get("record", {}).get("actor") == actor]
        if min_confidence is not None:
            results = [r for r in results
                       if r.get("record", {}).get("confidence", 1.0) >= min_confidence]
        return results

    def prompt_context(
        self,
        query: str,
        actor: Optional[str] = None,
        limit: int = 10,
        max_tokens: Optional[int] = None,
        fmt: str = "markdown",
    ) -> str:
        """Return memory as a formatted context block ready to inject into an LLM prompt.

        Args:
            query:      What to retrieve.
            actor:      Optional actor filter.
            limit:      Max memories to include.
            max_tokens: Truncate output to this many tokens (1 token ≈ 4 chars).
            fmt:        "markdown" (default) | "plain" | "xml"

        Returns:
            Formatted string, e.g.:
            "Relevant memories:\\n- **[decided]** Use PostgreSQL ..."
        """
        payload: Dict[str, Any] = {"query": query, "limit": limit, "format": fmt}
        if actor is not None:
            payload["actor"] = actor
        if max_tokens is not None:
            payload["max_tokens"] = max_tokens
        resp = self._session.post(
            f"{self.base_url}/memory/context", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json().get("context", "")

    def remember_and_recall(
        self,
        text: str,
        query: Optional[str] = None,
        actor: Optional[str] = None,
        session_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Store a memory and retrieve related memories in one call.

        Useful for chat agents: store the latest message and get context simultaneously.

        Returns:
            Dict with "stored" (ingest result) and "related" (list of related memories).
        """
        stored = self.remember(text, actor=actor, session_id=session_id)
        related = self.recall(query or text, actor=actor or stored.get("actor"), limit=5)
        return {"stored": stored, "related": related}

    def ping_latency_ms(self) -> float:
        """Return round-trip latency to /health in milliseconds."""
        t0 = time.perf_counter()
        self.health()
        return (time.perf_counter() - t0) * 1000

    def can_execute(self, action: str = "rollout") -> bool:
        """Query SelfModel gate: GET /self/can-execute?operation=...

        Falls back to /health ok only if self route is missing (404).
        """
        try:
            resp = self._session.get(
                f"{self.base_url}/self/can-execute",
                params={"operation": action},
                timeout=self.timeout,
            )
            if resp.status_code == 200:
                body = resp.json()
                if "should_execute" in body:
                    return bool(body["should_execute"])
                return False
            if resp.status_code == 404:
                status = self.health()
                return status.get("status") == "ok"
            return False
        except Exception:
            return False

    def rollout(
        self,
        initial_state: Any,
        actions: List[str],
        max_depth: int = 5,
    ) -> Dict[str, Any]:
        """Multi-step predict via POST /worldmodel/rollout {initial_state, actions}.

        ``initial_state`` may be a string or JSON-serializable object (stringified).
        """
        if not isinstance(initial_state, str):
            initial_state = json.dumps(initial_state)
        payload = {
            "initial_state": initial_state,
            "actions": actions,
            "max_depth": max_depth,
        }
        resp = self._session.post(
            f"{self.base_url}/worldmodel/rollout", json=payload, timeout=self.timeout
        )
        resp.raise_for_status()
        return resp.json()

