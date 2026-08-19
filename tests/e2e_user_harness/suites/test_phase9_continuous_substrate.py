"""E2E Phase 9 — Continuous Substrate: DigitalTwin + ExperienceStore REST surface.

Requires a running HipCortex server (HIPCORTEX_URL or default http://127.0.0.1:3030).
Skip automatically when server is unavailable.
"""
import os
import pytest

LIVE = os.environ.get("HIPCORTEX_LIVE_TESTS", "0") != "0"
BASE = os.environ.get("HIPCORTEX_URL", "http://127.0.0.1:3030")


def _client():
    import httpx
    return httpx.Client(base_url=BASE, timeout=10.0)


pytestmark = pytest.mark.skipif(not LIVE, reason="set HIPCORTEX_LIVE_TESTS=1 to run live E2E")


# ---------------------------------------------------------------------------
# DigitalTwin
# ---------------------------------------------------------------------------

class TestDigitalTwinEndpoints:
    def test_twin_create_returns_twin_id(self):
        with _client() as c:
            resp = c.post("/v1/twin", json={"dim": 4, "dt": 0.1, "max_covariance": 100.0})
        assert resp.status_code == 200
        data = resp.json()
        assert "twin_id" in data
        assert data["dim"] == 4

    def test_twin_step_returns_state_vector(self):
        with _client() as c:
            twin_id = c.post("/v1/twin", json={"dim": 3}).json()["twin_id"]
            resp = c.post(f"/v1/twin/{twin_id}/step", json={"action": "noop"})
        assert resp.status_code == 200
        state = resp.json()["state"]
        assert len(state) == 3
        assert all(isinstance(v, float) for v in state)

    def test_twin_rollout_returns_trajectory(self):
        with _client() as c:
            twin_id = c.post("/v1/twin", json={"dim": 2}).json()["twin_id"]
            resp = c.post(f"/v1/twin/{twin_id}/rollout", json={"actions": ["a", "b", "c"]})
        assert resp.status_code == 200
        data = resp.json()
        assert "continuous_trajectory" in data
        assert len(data["continuous_trajectory"]) == 3
        assert "continuous_sigma_norm" in data
        assert data["continuous_sigma_norm"] >= 0.0

    def test_twin_get_returns_state(self):
        with _client() as c:
            twin_id = c.post("/v1/twin", json={"dim": 2}).json()["twin_id"]
            c.post(f"/v1/twin/{twin_id}/step", json={"action": "act"})
            resp = c.get(f"/v1/twin/{twin_id}")
        assert resp.status_code == 200
        data = resp.json()
        assert data["twin_id"] == twin_id
        assert data["trajectory_steps"] == 1

    def test_twin_not_found_returns_404(self):
        with _client() as c:
            resp = c.get("/v1/twin/00000000-0000-0000-0000-000000000000")
        assert resp.status_code == 404

    def test_twin_step_invalid_id_returns_400(self):
        with _client() as c:
            resp = c.post("/v1/twin/not-a-uuid/step", json={"action": "x"})
        assert resp.status_code == 400


# ---------------------------------------------------------------------------
# ExperienceStore
# ---------------------------------------------------------------------------

class TestExperienceStoreEndpoints:
    def test_experience_tiers_returns_counts(self):
        with _client() as c:
            resp = c.get("/v1/experience/phase9-agent/tiers")
        assert resp.status_code == 200
        data = resp.json()
        for key in ("raw", "episode", "abstract", "compression_ratio"):
            assert key in data, f"missing key: {key}"
        assert data["raw"] >= 0
        assert data["episode"] >= 0
        assert data["abstract"] >= 0

    def test_experience_search_returns_results_list(self):
        with _client() as c:
            # seed a record so search has something to work with
            c.post("/memory/add", json={
                "actor": "phase9-agent",
                "action": "coded",
                "target": "substrate.rs",
                "record_type": "Temporal",
            })
            resp = c.post("/v1/experience/phase9-agent/search", json={"query": "substrate"})
        assert resp.status_code == 200
        data = resp.json()
        assert "results" in data
        assert isinstance(data["results"], list)

    def test_experience_search_missing_query_returns_400(self):
        with _client() as c:
            resp = c.post("/v1/experience/phase9-agent/search", json={})
        assert resp.status_code == 400
