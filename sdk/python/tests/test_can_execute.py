"""can_execute uses /self/can-execute when available."""
from unittest.mock import MagicMock

from hipcortex.client import HipCortexClient


def test_can_execute_uses_self_route():
    client = HipCortexClient(base_url="http://127.0.0.1:3030")
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    mock_resp.json.return_value = {
        "operation": "rollout",
        "should_execute": True,
        "confidence": 0.9,
    }
    client._session = MagicMock()
    client._session.get.return_value = mock_resp

    assert client.can_execute("rollout") is True
    client._session.get.assert_called_once()
    args, kwargs = client._session.get.call_args
    assert args[0].endswith("/self/can-execute")
    assert kwargs["params"]["operation"] == "rollout"


def test_can_execute_false_when_gate_rejects():
    client = HipCortexClient(base_url="http://127.0.0.1:3030")
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    mock_resp.json.return_value = {"should_execute": False, "rationale": "busy"}
    client._session = MagicMock()
    client._session.get.return_value = mock_resp
    assert client.can_execute("heavy_op") is False
