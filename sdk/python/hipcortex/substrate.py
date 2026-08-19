"""HipCortex Cognitive Substrate — DigitalTwin + ExperienceStore SDK façade."""

from __future__ import annotations

from typing import Any, Dict, List, Optional

import requests


class HipCortexSubstrate:
    """High-level façade for DigitalTwin and ExperienceStore REST endpoints.

    Args:
        base_url: HipCortex server URL (e.g. ``http://localhost:3030``).
        timeout:  Per-request timeout in seconds.
        api_key:  Optional X-Api-Key header value.
    """

    def __init__(
        self,
        base_url: str = "http://localhost:3030",
        timeout: float = 10.0,
        api_key: Optional[str] = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self._session = requests.Session()
        if api_key:
            self._session.headers["X-Api-Key"] = api_key

    # ------------------------------------------------------------------
    # DigitalTwin
    # ------------------------------------------------------------------

    def create_twin(
        self,
        dim: int = 4,
        dt: float = 0.1,
        max_covariance: float = 100.0,
    ) -> str:
        """Create a DigitalTwin and return its ``twin_id``."""
        resp = self._session.post(
            f"{self.base_url}/v1/twin",
            json={"dim": dim, "dt": dt, "max_covariance": max_covariance},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()["twin_id"]

    def twin_step(self, twin_id: str, action: str) -> List[float]:
        """Advance a DigitalTwin by one action, returning the new state vector."""
        resp = self._session.post(
            f"{self.base_url}/v1/twin/{twin_id}/step",
            json={"action": action},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()["state"]

    def twin_rollout(self, twin_id: str, actions: List[str]) -> Dict[str, Any]:
        """Run a multi-step hybrid rollout on a DigitalTwin."""
        resp = self._session.post(
            f"{self.base_url}/v1/twin/{twin_id}/rollout",
            json={"actions": actions},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def twin_get(self, twin_id: str) -> Dict[str, Any]:
        """Retrieve trajectory and record count for a DigitalTwin."""
        resp = self._session.get(
            f"{self.base_url}/v1/twin/{twin_id}",
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    # ------------------------------------------------------------------
    # ExperienceStore
    # ------------------------------------------------------------------

    def experience_tiers(self, actor: str) -> Dict[str, Any]:
        """Return raw/episode/abstract counts and compression ratio."""
        resp = self._session.get(
            f"{self.base_url}/v1/experience/{actor}/tiers",
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json()

    def experience_search(self, actor: str, query: str) -> List[Dict[str, Any]]:
        """Search the compressed experience pyramid for an actor."""
        resp = self._session.post(
            f"{self.base_url}/v1/experience/{actor}/search",
            json={"query": query},
            timeout=self.timeout,
        )
        resp.raise_for_status()
        return resp.json().get("results", [])
