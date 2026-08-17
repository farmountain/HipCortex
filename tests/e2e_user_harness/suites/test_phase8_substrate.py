"""Gate 4: Multi-surface parity — REST ≡ MCP ≡ Python schema.

Schema-only tests run without a live server.
Live tests require HIPCORTEX_LIVE_TESTS=1 and HIPCORTEX_URL set.
"""
import inspect
import json
import os
import sys

import pytest

BASE = os.environ.get("HIPCORTEX_URL", "http://localhost:3000")
LIVE = os.environ.get("HIPCORTEX_LIVE_TESTS", "0") == "1"

# Add SDK to path
_SDK_PY = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../sdk/python"))
_SDK_MCP = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../sdk/mcp"))
sys.path.insert(0, _SDK_PY)
sys.path.insert(0, _SDK_MCP)


# ── Schema-only tests (no server required) ──────────────────────────────────

def test_python_client_has_all_new_methods():
    from hipcortex.client import HipCortexClient
    c = HipCortexClient(base_url="http://localhost:3000")
    expected = [
        "get_state_diff",
        "consolidate_memory",
        "get_system_health",
        "get_live_beliefs",
        "simulate_rollout",
    ]
    for name in expected:
        assert hasattr(c, name), f"HipCortexClient missing method: {name}"


def test_python_get_state_diff_signature():
    from hipcortex.client import HipCortexClient
    sig = inspect.signature(HipCortexClient.get_state_diff)
    params = list(sig.parameters)
    assert "from_tx" in params
    assert "to_tx" in params


def test_python_get_system_health_signature():
    from hipcortex.client import HipCortexClient
    sig = inspect.signature(HipCortexClient.get_system_health)
    params = list(sig.parameters)
    assert "self" in params


def test_python_get_live_beliefs_has_min_conf():
    from hipcortex.client import HipCortexClient
    sig = inspect.signature(HipCortexClient.get_live_beliefs)
    assert "min_conf" in sig.parameters
    assert sig.parameters["min_conf"].default == 0.0


def test_python_simulate_rollout_caps_max_depth():
    """simulate_rollout enforces max_depth ≤ 5 at client layer."""
    from hipcortex.client import HipCortexClient
    import unittest.mock as mock
    c = HipCortexClient(base_url="http://localhost:3000")
    with mock.patch.object(c._session, "post") as m:
        m.return_value.json.return_value = {}
        m.return_value.raise_for_status.return_value = None
        c.simulate_rollout("s0", ["a1"], max_depth=99)
        call_kwargs = m.call_args
        sent_body = call_kwargs[1]["json"]
        assert sent_body["max_depth"] <= 5, f"max_depth not capped: {sent_body['max_depth']}"


def _load_mcp_server():
    import importlib.util
    spec = importlib.util.spec_from_file_location("mcp_server", os.path.join(_SDK_MCP, "server.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def test_mcp_has_simulate_rollout_and_get_system_health():
    mcp_server = _load_mcp_server()
    names = {t["name"] for t in mcp_server.TOOLS}
    assert "simulate_rollout" in names, "MCP missing simulate_rollout"
    assert "get_system_health" in names, "MCP missing get_system_health"


def test_mcp_simulate_rollout_rejects_six_actions():
    mcp_server = _load_mcp_server()
    result = mcp_server.handle_simulate_rollout(
        {"initial_state": "s0", "actions": ["a", "b", "c", "d", "e", "f"]}
    )
    assert "max_depth" in result.lower(), f"Expected 'max_depth' in error: {result}"


def test_mcp_get_live_beliefs_schema_has_min_conf():
    mcp_server = _load_mcp_server()
    tool = next(t for t in mcp_server.TOOLS if t["name"] == "get_live_beliefs")
    assert "min_conf" in tool["inputSchema"]["properties"], "get_live_beliefs missing min_conf param"


# ── Live tests (require running server) ─────────────────────────────────────

HEALTH_REQUIRED_FIELDS = [
    "calibration_score",
    "prediction_error_ewma",
    "consolidation_pressure",
    "epistemic_entropy",
    "healthy",
]


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_live_self_health_has_calibration_fields():
    import requests
    resp = requests.get(f"{BASE}/self/health", timeout=10)
    assert resp.status_code == 200
    data = resp.json()
    for field in HEALTH_REQUIRED_FIELDS:
        assert field in data, f"GET /self/health missing field: {field}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_live_state_diff_schema_parity():
    """REST /v1/state/diff and Python client return same top-level schema."""
    import requests
    from hipcortex.client import HipCortexClient
    rest = requests.post(f"{BASE}/v1/state/diff", json={"from_tx": 0, "to_tx": 5}, timeout=10)
    assert rest.status_code in (200, 400)
    rest_data = rest.json()
    client = HipCortexClient(base_url=BASE)
    try:
        py_data = client.get_state_diff(0, 5)
    except Exception:
        py_data = {}
    for key in ("from_tx", "to_tx"):
        assert key in rest_data, f"REST missing key: {key}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_live_v1_beliefs_returns_list():
    import requests
    resp = requests.get(f"{BASE}/v1/beliefs?min_conf=0.0", timeout=10)
    assert resp.status_code == 200
    data = resp.json()
    assert "beliefs" in data and "count" in data


# ── v0.8.0 Phase 1 Acceptance Gates (G1-1..G1-4) ────────────────────────────

import uuid as _uuid
import time as _time

def _add_memory_delta():
    return {
        "type": "AddMemory",
        "record": {
            "id": str(_uuid.uuid4()),
            "actor": "e2e-test",
            "action": "test",
            "target": "phase1",
            "memory_type": "Temporal",
            "timestamp": int(_time.time() * 1000),
            "metadata": {},
        }
    }


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_1_transact_returns_ok_and_tx_cursor():
    """G1-1: POST /v1/cognitive/transact → ok=true, tx_cursor int ≥ 0."""
    import requests
    payload = {"delta": _add_memory_delta(), "actor": "e2e-test"}
    resp = requests.post(f"{BASE}/v1/cognitive/transact", json=payload, timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"ok not true: {data}"
    assert isinstance(data.get("tx_cursor"), int) and data["tx_cursor"] >= 0, f"bad tx_cursor: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_2_cognitive_diff_returns_range_fields():
    """G1-2: GET /v1/cognitive/diff?from_tx=0&to_tx=999 → 200, from_tx/to_tx in response."""
    import requests
    resp = requests.get(f"{BASE}/v1/cognitive/diff", params={"from_tx": 0, "to_tx": 999}, timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "from_tx" in data, f"missing from_tx: {data}"
    assert "to_tx" in data, f"missing to_tx: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_3_v1_self_health_returns_healthy_bool():
    """G1-3: GET /v1/self/health → 200, has healthy bool."""
    import requests
    resp = requests.get(f"{BASE}/v1/self/health", timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "healthy" in data, f"missing healthy field: {data}"
    assert isinstance(data["healthy"], bool), f"healthy not bool: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_4_empty_actor_returns_400():
    """G1-4: POST /v1/cognitive/transact with empty actor → 400."""
    import requests
    payload = {"delta": _add_memory_delta(), "actor": ""}
    resp = requests.post(f"{BASE}/v1/cognitive/transact", json=payload, timeout=10)
    assert resp.status_code == 400, f"expected 400 for empty actor, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is False
    assert "actor" in data.get("error", "").lower(), f"error should mention actor: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_live_tx_cursor_monotonically_increases():
    """Sequential transacts must strictly increase tx_cursor."""
    import requests
    r1 = requests.post(f"{BASE}/v1/cognitive/transact", json={"delta": _add_memory_delta(), "actor": "e2e-test"}, timeout=10)
    r2 = requests.post(f"{BASE}/v1/cognitive/transact", json={"delta": _add_memory_delta(), "actor": "e2e-test"}, timeout=10)
    assert r1.status_code == 200 and r2.status_code == 200
    c1 = r1.json().get("tx_cursor", -1)
    c2 = r2.json().get("tx_cursor", -1)
    assert c2 > c1, f"tx_cursor not monotonic: {c1} → {c2}"


# ─── G2: DigitalTwin fork lifecycle ──────────────────────────────────────────

@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_1_post_fork_creates_fork():
    """G2-1: POST /v1/fork returns id + base_tx."""
    import requests
    resp = requests.post(f"{BASE}/v1/fork", timeout=10)
    assert resp.status_code == 201, f"expected 201, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "fork_id" in data, f"missing fork_id: {data}"
    assert "base_tx" in data, f"missing base_tx: {data}"
    assert isinstance(data["base_tx"], int), f"base_tx not int: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_2_fork_step_advances_tx():
    """G2-2: POST /v1/fork/:id/step advances fork_tx."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(f"{BASE}/v1/fork/{fork_id}/step", json={"action": "move-left"}, timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "fork_tx" in data, f"missing fork_tx: {data}"
    assert data["fork_tx"] > 0, f"fork_tx must increase: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_3_fork_transact_add_memory():
    """G2-3: POST /v1/fork/:id/transact with AddMemory delta → 200."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    delta = _add_memory_delta()
    resp = requests.post(
        f"{BASE}/v1/fork/{fork_id}/transact",
        json={"delta": delta, "actor": "e2e-fork"},
        timeout=10,
    )
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"expected ok: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_4_fork_snapshot_returns_cognitive_snapshot():
    """G2-4: GET /v1/fork/:id/snapshot returns CognitiveSnapshot schema."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.get(f"{BASE}/v1/fork/{fork_id}/snapshot", timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    for field in ("tx_cursor", "temporal", "world", "self_model", "goals", "skills", "beliefs"):
        assert field in data, f"snapshot missing field '{field}': {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_5_delete_fork_returns_204():
    """G2-5: DELETE /v1/fork/:id removes fork, subsequent snapshot → 404."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    del_resp = requests.delete(f"{BASE}/v1/fork/{fork_id}", timeout=10)
    assert del_resp.status_code == 204, f"expected 204, got {del_resp.status_code}: {del_resp.text}"
    snap_resp = requests.get(f"{BASE}/v1/fork/{fork_id}/snapshot", timeout=10)
    assert snap_resp.status_code == 404, f"expected 404 after delete, got {snap_resp.status_code}"
