"""Unit tests for HipCortexSubstrate — mocked HTTP responses."""
from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest

from hipcortex.substrate import HipCortexSubstrate


def _mock_resp(json_data: dict, status: int = 200) -> MagicMock:
    r = MagicMock()
    r.status_code = status
    r.ok = status < 400
    r.json.return_value = json_data
    r.raise_for_status = MagicMock()
    return r


@pytest.fixture()
def substrate():
    return HipCortexSubstrate(base_url="http://hc-test:3030")


class TestDigitalTwin:
    def test_create_twin_returns_twin_id(self, substrate):
        twin_id = "00000000-0000-0000-0000-000000000001"
        with patch.object(substrate._session, "post", return_value=_mock_resp({"twin_id": twin_id, "dim": 4})) as mock:
            result = substrate.create_twin(dim=4, dt=0.1, max_covariance=50.0)
        assert result == twin_id
        mock.assert_called_once_with(
            "http://hc-test:3030/v1/twin",
            json={"dim": 4, "dt": 0.1, "max_covariance": 50.0},
            timeout=10.0,
        )

    def test_twin_step_returns_state(self, substrate):
        twin_id = "abc"
        state = [1.0, 2.0, 3.0, 4.0]
        with patch.object(substrate._session, "post", return_value=_mock_resp({"state": state})):
            result = substrate.twin_step(twin_id, "move")
        assert result == state

    def test_twin_rollout_returns_result(self, substrate):
        twin_id = "abc"
        expected = {"trajectory": [[1.0], [2.0]], "continuous_halted": False, "continuous_sigma_norm": 0.5}
        with patch.object(substrate._session, "post", return_value=_mock_resp(expected)):
            result = substrate.twin_rollout(twin_id, ["a", "b"])
        assert result["continuous_halted"] is False

    def test_twin_get_returns_dict(self, substrate):
        twin_id = "abc"
        expected = {"twin_id": twin_id, "trajectory_steps": 2, "records_count": 3}
        with patch.object(substrate._session, "get", return_value=_mock_resp(expected)):
            result = substrate.twin_get(twin_id)
        assert result["trajectory_steps"] == 2


class TestExperienceStore:
    def test_experience_tiers_returns_counts(self, substrate):
        payload = {"raw": 100, "episode": 10, "abstract": 2, "compression_ratio": 0.02, "raw_pressure": 0.1}
        with patch.object(substrate._session, "get", return_value=_mock_resp(payload)):
            result = substrate.experience_tiers("agent-1")
        assert result["raw"] == 100
        assert result["compression_ratio"] == pytest.approx(0.02)

    def test_experience_search_returns_list(self, substrate):
        records = [{"id": "r1", "action": "coded", "target": "auth.rs"}]
        with patch.object(substrate._session, "post", return_value=_mock_resp({"count": 1, "results": records})):
            result = substrate.experience_search("agent-1", "auth")
        assert len(result) == 1
        assert result[0]["action"] == "coded"
