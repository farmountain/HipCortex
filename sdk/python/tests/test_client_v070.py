"""Unit tests for HipCortexClient v0.7.0 new methods.

All tests are offline — HTTP calls mocked via unittest.mock.
"""
from __future__ import annotations

import unittest
from unittest import mock

from hipcortex.client import HipCortexClient


def _make_client() -> HipCortexClient:
    return HipCortexClient(base_url="http://localhost:3000")


def _mock_get(return_value: dict):
    m = mock.MagicMock()
    m.json.return_value = return_value
    m.raise_for_status.return_value = None
    return m


def _mock_post(return_value: dict):
    m = mock.MagicMock()
    m.json.return_value = return_value
    m.raise_for_status.return_value = None
    return m


class TestGetStateDiff(unittest.TestCase):
    def test_posts_to_correct_endpoint(self):
        c = _make_client()
        payload = {"from_tx": 0, "to_tx": 5, "memory_delta": {}}
        with mock.patch.object(c._session, "post", return_value=_mock_post(payload)) as m:
            result = c.get_state_diff(0, 5)
            m.assert_called_once()
            call_kwargs = m.call_args
            assert "/v1/state/diff" in call_kwargs[0][0]
            assert call_kwargs[1]["json"] == {"from_tx": 0, "to_tx": 5}
        assert result == payload

    def test_returns_json(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post({"from_tx": 1, "to_tx": 2})):
            result = c.get_state_diff(1, 2)
        assert "from_tx" in result


class TestConsolidateMemory(unittest.TestCase):
    def test_posts_min_group_size(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post({"consolidated": 3})) as m:
            c.consolidate_memory(min_group_size=5)
            call_kwargs = m.call_args
            assert "/v1/memory/consolidate" in call_kwargs[0][0]
            assert call_kwargs[1]["json"]["min_group_size"] == 5

    def test_default_min_group_size_is_3(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post({})) as m:
            c.consolidate_memory()
            assert m.call_args[1]["json"]["min_group_size"] == 3


class TestGetSystemHealth(unittest.TestCase):
    HEALTH_RESPONSE = {
        "healthy": True,
        "overall": 0.95,
        "calibration_score": 0.92,
        "prediction_error_ewma": 0.05,
        "consolidation_pressure": 0.18,
        "epistemic_entropy": 1.2,
        "calibration_healthy": True,
    }

    def test_gets_self_health_endpoint(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get(self.HEALTH_RESPONSE)) as m:
            result = c.get_system_health()
            assert "/self/health" in m.call_args[0][0]
        assert result["calibration_score"] == 0.92

    def test_returns_all_calibration_fields(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get(self.HEALTH_RESPONSE)):
            result = c.get_system_health()
        for field in ("calibration_score", "prediction_error_ewma", "consolidation_pressure", "epistemic_entropy"):
            assert field in result, f"missing field: {field}"


class TestGetLiveBeliefs(unittest.TestCase):
    def test_passes_min_conf_param(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get({"beliefs": []})) as m:
            c.get_live_beliefs(min_conf=0.8)
            params = m.call_args[1]["params"]
            assert params["min_conf"] == 0.8

    def test_default_min_conf_is_zero(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get({})) as m:
            c.get_live_beliefs()
            assert m.call_args[1]["params"]["min_conf"] == 0.0

    def test_passes_actor_when_given(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get({})) as m:
            c.get_live_beliefs(actor="test-agent")
            assert m.call_args[1]["params"]["actor"] == "test-agent"

    def test_omits_actor_when_none(self):
        c = _make_client()
        with mock.patch.object(c._session, "get", return_value=_mock_get({})) as m:
            c.get_live_beliefs()
            assert "actor" not in m.call_args[1]["params"]


class TestSimulateRollout(unittest.TestCase):
    ROLLOUT_RESPONSE = {
        "predicted_state": "s1",
        "mode": "dirichlet",
        "uncertainty": 0.12,
    }

    def test_caps_max_depth_at_5(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post(self.ROLLOUT_RESPONSE)) as m:
            c.simulate_rollout("s0", ["a1"], max_depth=99)
            sent = m.call_args[1]["json"]
            assert sent["max_depth"] <= 5, f"max_depth not capped: {sent['max_depth']}"

    def test_accepts_depth_5(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post(self.ROLLOUT_RESPONSE)) as m:
            c.simulate_rollout("s0", ["a1", "a2"], max_depth=5)
            assert m.call_args[1]["json"]["max_depth"] == 5

    def test_posts_to_rollout_endpoint(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post(self.ROLLOUT_RESPONSE)) as m:
            c.simulate_rollout("s0", ["move"])
            assert "/worldmodel/rollout" in m.call_args[0][0]

    def test_dict_initial_state_serialized(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post(self.ROLLOUT_RESPONSE)) as m:
            c.simulate_rollout({"x": 1}, ["a"])
            sent = m.call_args[1]["json"]
            assert isinstance(sent["initial_state"], str)

    def test_returns_rollout_response(self):
        c = _make_client()
        with mock.patch.object(c._session, "post", return_value=_mock_post(self.ROLLOUT_RESPONSE)):
            result = c.simulate_rollout("s0", ["a"])
        assert result["predicted_state"] == "s1"
        assert result["uncertainty"] == 0.12


if __name__ == "__main__":
    unittest.main()
