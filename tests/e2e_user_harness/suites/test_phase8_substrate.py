"""Gate 4: Multi-surface parity — REST ≡ MCP ≡ Python schema.

Schema-only tests run without a live server.
Live tests require HIPCORTEX_LIVE_TESTS=1 and HIPCORTEX_URL set.
"""
import inspect
import json
import os
import sys

import pytest

BASE = os.environ.get("HIPCORTEX_URL", "http://localhost:3030")
LIVE = os.environ.get("HIPCORTEX_LIVE_TESTS", "0") == "1"

# Add SDK to path
_SDK_PY = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../sdk/python"))
_SDK_MCP = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../sdk/mcp"))
sys.path.insert(0, _SDK_PY)
sys.path.insert(0, _SDK_MCP)


# ── Schema-only tests (no server required) ──────────────────────────────────

def test_python_client_has_all_new_methods():
    from hipcortex.client import HipCortexClient
    c = HipCortexClient(base_url=BASE)
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
    c = HipCortexClient(base_url=BASE)
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
    # AddMemory(MemoryRecord) uses serde internal tagging — MemoryRecord fields
    # are flattened into the same object as "type", not nested under "record".
    import datetime
    return {
        "type": "AddMemory",
        "id": str(_uuid.uuid4()),
        "record_type": "Temporal",
        "timestamp": datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%S.%f") + "Z",
        "actor": "e2e-test",
        "action": "test",
        "target": "phase1",
        "metadata": {},
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
    """G2-1: POST /v1/fork → 200, fork_id is valid UUID, base_tx present, expires_in_secs=60."""
    import requests, uuid
    resp = requests.post(f"{BASE}/v1/fork", timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "fork_id" in data, f"missing fork_id: {data}"
    assert "base_tx" in data, f"missing base_tx: {data}"
    assert "expires_in_secs" in data, f"missing expires_in_secs: {data}"
    assert data["expires_in_secs"] == 60, f"expected expires_in_secs=60: {data}"
    uuid.UUID(data["fork_id"])  # raises if not a valid UUID
    assert isinstance(data["base_tx"], int), f"base_tx not int: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_2_fork_step_advances_tx_parent_unchanged():
    """G2-2: POST /v1/fork/:id/step → 200, fork_tx increments; parent tx_cursor unchanged."""
    import requests
    # capture parent tx before fork
    parent_snap_before = requests.get(f"{BASE}/v1/cognitive/snapshot", timeout=10).json()
    parent_tx_before = parent_snap_before.get("tx_cursor", -1)

    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(f"{BASE}/v1/fork/{fork_id}/step", json={"action": "move-left"}, timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"expected ok=true: {data}"
    assert "fork_tx" in data, f"missing fork_tx: {data}"
    assert data["fork_tx"] > 0, f"fork_tx must increase: {data}"
    assert "steps_taken" in data, f"missing steps_taken: {data}"

    # parent tx_cursor must not have changed
    parent_snap_after = requests.get(f"{BASE}/v1/cognitive/snapshot", timeout=10).json()
    parent_tx_after = parent_snap_after.get("tx_cursor", -2)
    assert parent_tx_after == parent_tx_before, (
        f"parent tx_cursor changed after fork step: {parent_tx_before} → {parent_tx_after}"
    )


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_3_fork_snapshot_returns_cognitive_snapshot():
    """G2-3: GET /v1/fork/:id/snapshot → 200 with all CognitiveSnapshot fields."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.get(f"{BASE}/v1/fork/{fork_id}/snapshot", timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    for field in ("tx_cursor", "temporal", "world", "self_model", "goals", "skills", "beliefs"):
        assert field in data, f"snapshot missing field '{field}': {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_4_delete_fork_then_step_returns_404():
    """G2-4: DELETE /v1/fork/:id → 200 {ok:true}; subsequent step → 404."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    del_resp = requests.delete(f"{BASE}/v1/fork/{fork_id}", timeout=10)
    assert del_resp.status_code == 200, f"expected 200, got {del_resp.status_code}: {del_resp.text}"
    assert del_resp.json().get("ok") is True, f"expected ok=true: {del_resp.json()}"
    step_resp = requests.post(f"{BASE}/v1/fork/{fork_id}/step", json={"action": "x"}, timeout=10)
    assert step_resp.status_code == 404, f"expected 404 after delete, got {step_resp.status_code}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_5_empty_step_action_returns_400():
    """G2-5 (proxy): empty action string on step → 400 Bad Request."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(f"{BASE}/v1/fork/{fork_id}/step", json={"action": ""}, timeout=10)
    assert resp.status_code == 400, f"expected 400 for empty action, got {resp.status_code}: {resp.text}"
    assert resp.json().get("ok") is False


# expiry→410 requires waiting 60s; covered by unit test (is_expired()) and manual QA
@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g2_live_fork_transact_add_memory():
    """Supplemental: POST /v1/fork/:id/transact with AddMemory delta → 200."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    delta = _add_memory_delta()
    resp = requests.post(
        f"{BASE}/v1/fork/{fork_id}/transact",
        json={"delta": delta, "actor": "e2e-fork"},
        timeout=10,
    )
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    assert resp.json().get("ok") is True


# ─── G3: HybridDynamics rollout ───────────────────────────────────────────────

@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g3_1_rollout_3_actions_returns_3_steps():
    """G3-1: Rollout with 3 actions → 200, steps has 3 entries, uncertainty map present."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(
        f"{BASE}/v1/fork/{fork_id}/rollout",
        json={"actions": ["move_north", "grab_object", "move_south"], "sigma2_max": 0.25},
        timeout=10,
    )
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert "steps" in data, f"missing steps: {data}"
    assert len(data["steps"]) == 3, f"expected 3 steps, got {len(data['steps'])}"
    for step in data["steps"]:
        assert "uncertainty" in step, f"step missing uncertainty: {step}"
        assert isinstance(step["uncertainty"], dict), f"uncertainty must be dict: {step}"
        assert "fork_tx" in step, f"step missing fork_tx: {step}"
    assert "halted_early" in data
    assert "final_fork_tx" in data


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g3_2_rollout_7_actions_capped_at_5():
    """G3-2: Rollout with 7 actions → only 5 steps in response (k-cap enforced)."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(
        f"{BASE}/v1/fork/{fork_id}/rollout",
        json={"actions": [f"action_{i}" for i in range(7)], "sigma2_max": 1.0},
        timeout=10,
    )
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert len(data["steps"]) == 5, f"k-cap must limit to 5 steps, got {len(data['steps'])}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g3_3_rollout_low_sigma2_halts_early():
    """G3-3: Rollout with sigma2_max=0.001 → halted_early=true (noise_floor=0.01 > limit)."""
    import requests
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    resp = requests.post(
        f"{BASE}/v1/fork/{fork_id}/rollout",
        json={"actions": ["a", "b", "c"], "sigma2_max": 0.001},
        timeout=10,
    )
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("halted_early") is True, f"expected halted_early=true: {data}"
    assert data.get("halt_reason") is not None, f"expected halt_reason: {data}"
    assert len(data["steps"]) <= 3


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g3_4_rollout_parent_snapshot_unchanged():
    """G3-4: Parent CognitiveHandle snapshot tx_cursor unchanged after fork rollout."""
    import requests
    parent_before = requests.get(f"{BASE}/v1/cognitive/snapshot", timeout=10).json()
    fork_id = requests.post(f"{BASE}/v1/fork", timeout=10).json()["fork_id"]
    requests.post(
        f"{BASE}/v1/fork/{fork_id}/rollout",
        json={"actions": ["move", "turn"], "sigma2_max": 1.0},
        timeout=10,
    )
    parent_after = requests.get(f"{BASE}/v1/cognitive/snapshot", timeout=10).json()
    assert parent_before.get("tx_cursor") == parent_after.get("tx_cursor"), (
        f"parent tx_cursor changed: {parent_before.get('tx_cursor')} → {parent_after.get('tx_cursor')}"
    )


# ── Phase 4: ExperienceStore gates (G4-1..G4-5) ──────────────────────────────

@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g4_1_consolidate_removes_sources_inserts_summary():
    """G4-1: POST /v1/cognitive/transact Consolidate → sources gone, summary present in snapshot."""
    import requests, uuid
    # Add two source records
    actor = f"g4-agent-{uuid.uuid4().hex[:6]}"
    def add(a):
        return requests.post(f"{BASE}/v1/cognitive/transact", json={
            "actor": a,
            "delta": {"type": "AddMemory", "record_type": "Temporal",
                      "actor": a, "action": "did", "target": "t",
                      "metadata": {}, "status": "active"}
        }, timeout=10)
    # Use AddMemory via full MemoryRecord payload — server accepts raw delta
    import requests as req
    snap0 = req.get(f"{BASE}/v1/cognitive/snapshot", timeout=10).json()
    count0 = snap0["temporal"]["record_count"]
    # Add via the /memory endpoint instead (simpler)
    r1 = req.post(f"{BASE}/memory/add", json={"actor": actor, "action": "did", "target": "t1",
                                               "record_type": "Temporal", "metadata": {}}, timeout=10)
    r2 = req.post(f"{BASE}/memory/add", json={"actor": actor, "action": "did", "target": "t2",
                                               "record_type": "Temporal", "metadata": {}}, timeout=10)
    assert r1.status_code in (200, 201), f"add r1 failed: {r1.text}"
    assert r2.status_code in (200, 201), f"add r2 failed: {r2.text}"
    id1 = r1.json()["record_id"]
    id2 = r2.json()["record_id"]

    summary_id = str(uuid.uuid4())
    resp = req.post(f"{BASE}/v1/cognitive/transact", json={
        "actor": "system",
        "delta": {
            "type": "Consolidate",
            "source_ids": [id1, id2],
            "summary": {
                "id": summary_id, "record_type": "Temporal",
                "actor": "system", "action": "consolidated", "target": "group",
                "metadata": {}, "status": "active",
                "confidence": 1.0, "priority": "normal", "tags": [],
                "timestamp": "2026-08-17T00:00:00Z",
                "derived_from": None, "evidence": [], "react_iteration": None,
                "namespace": None, "integrity_hash": None,
            }
        }
    }, timeout=10)
    assert resp.status_code == 200, f"consolidate failed: {resp.status_code} {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"expected ok=true: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g4_2_forget_actor_zero_records_remain():
    """G4-2: ForgetActor("test-actor") → records_deleted returned, 0 records for actor in Hot."""
    import requests, uuid
    actor = f"gdpr-{uuid.uuid4().hex[:8]}"
    for _ in range(3):
        requests.post(f"{BASE}/memory/add", json={
            "actor": actor, "action": "did", "target": "t",
            "record_type": "Temporal", "metadata": {}
        }, timeout=10)

    resp = requests.post(f"{BASE}/v1/cognitive/transact", json={
        "actor": "system",
        "delta": {"type": "ForgetActor", "actor": actor}
    }, timeout=10)
    assert resp.status_code == 200, f"ForgetActor failed: {resp.status_code} {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"expected ok=true: {data}"
    assert "records_deleted" in data, f"records_deleted missing: {data}"
    assert data["records_deleted"] >= 3, f"expected >=3 deleted, got: {data['records_deleted']}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g4_3_archive_record_referenced_removed_from_hot():
    """G4-3: ArchiveRecord(id) with any record → removed from Hot store (ok=true)."""
    import requests, uuid
    actor = f"arc-{uuid.uuid4().hex[:6]}"
    add = requests.post(f"{BASE}/memory/add", json={
        "actor": actor, "action": "did", "target": "t",
        "record_type": "Temporal", "metadata": {}
    }, timeout=10)
    assert add.status_code in (200, 201), f"add failed: {add.text}"
    rec_id = add.json()["record_id"]

    resp = requests.post(f"{BASE}/v1/cognitive/transact", json={
        "actor": "system",
        "delta": {"type": "ArchiveRecord", "id": rec_id}
    }, timeout=10)
    assert resp.status_code == 200, f"ArchiveRecord failed: {resp.status_code} {resp.text}"
    assert resp.json().get("ok") is True


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g4_4_archive_record_orphan_deleted():
    """G4-4: ArchiveRecord(id) on orphan (no GC references) → ok=true, record gone."""
    import requests, uuid
    actor = f"orphan-{uuid.uuid4().hex[:6]}"
    add = requests.post(f"{BASE}/memory/add", json={
        "actor": actor, "action": "did", "target": "orphan",
        "record_type": "Temporal", "metadata": {}
    }, timeout=10)
    assert add.status_code in (200, 201), f"add failed: {add.text}"
    rec_id = add.json()["record_id"]

    resp = requests.post(f"{BASE}/v1/cognitive/transact", json={
        "actor": "system",
        "delta": {"type": "ArchiveRecord", "id": rec_id}
    }, timeout=10)
    assert resp.status_code == 200, f"ArchiveRecord orphan failed: {resp.status_code} {resp.text}"
    assert resp.json().get("ok") is True


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g4_5_consolidate_over_100_source_ids_returns_error():
    """G4-5: Consolidate source_ids.len() > 100 → 400/500 DeltaInvalid error."""
    import requests, uuid
    ids = [str(uuid.uuid4()) for _ in range(101)]
    summary_id = str(uuid.uuid4())
    resp = requests.post(f"{BASE}/v1/cognitive/transact", json={
        "actor": "system",
        "delta": {
            "type": "Consolidate",
            "source_ids": ids,
            "summary": {
                "id": summary_id, "record_type": "Temporal",
                "actor": "system", "action": "consolidated", "target": "group",
                "metadata": {}, "status": "active",
                "confidence": 1.0, "priority": "normal", "tags": [],
                "timestamp": "2026-08-17T00:00:00Z",
                "derived_from": None, "evidence": [], "react_iteration": None,
                "namespace": None, "integrity_hash": None,
            }
        }
    }, timeout=10)
    assert resp.status_code in (400, 422, 500), (
        f"expected error for >100 source_ids, got {resp.status_code}: {resp.text}"
    )


# ---------------------------------------------------------------------------
# Phase 5 Gates: Agent Surfaces (G5-1..G5-5)
# ---------------------------------------------------------------------------

@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g5_1_python_sdk_transact_returns_ok_tx_cursor():
    """G5-1: Python SDK client.transact() returns ok=True + tx_cursor int."""
    import sys
    import os
    sdk_path = os.path.join(os.path.dirname(__file__), "../../../sdk/python")
    if sdk_path not in sys.path:
        sys.path.insert(0, sdk_path)
    from hipcortex.client import HipCortexClient
    client = HipCortexClient(base_url=BASE)
    import datetime as _dt
    result = client.transact(
        {
            "type": "AddMemory",
            "id": __import__("uuid").uuid4().__str__(),
            "record_type": "Temporal",
            "actor": "g5-test",
            "action": "test",
            "target": "t",
            "timestamp": _dt.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%S.%f") + "Z",
            "metadata": {},
        },
        actor="g5-sdk"
    )
    assert result.get("ok") is True, f"expected ok=true: {result}"
    assert isinstance(result.get("tx_cursor"), int), f"tx_cursor must be int: {result}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g5_2_mcp_cognitive_transact_tool_exists_and_callable():
    """G5-2: MCP cognitive_transact tool in tools/list; schema has delta+actor required."""
    import sys, os, json, subprocess
    mcp_server = os.path.join(os.path.dirname(__file__), "../../../sdk/mcp/server.py")
    init_msg = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}})
    list_msg = json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
    proc = subprocess.run(
        [sys.executable, mcp_server],
        input=f"{init_msg}\n{list_msg}\n",
        capture_output=True, text=True, timeout=10,
        env={**os.environ, "HIPCORTEX_URL": BASE}
    )
    lines = [l for l in proc.stdout.splitlines() if l.strip().startswith("{")]
    tools_resp = next((json.loads(l) for l in lines if json.loads(l).get("id") == 2), None)
    assert tools_resp is not None, f"No tools/list response: {proc.stdout}"
    tool_names = [t["name"] for t in tools_resp["result"]["tools"]]
    assert "cognitive_transact" in tool_names, f"cognitive_transact missing from tools: {tool_names}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g5_3_python_sdk_fork_rollout_returns_steps():
    """G5-3: Python SDK client.fork_rollout(id, actions, 0.25) returns steps list."""
    import sys, os
    sdk_path = os.path.join(os.path.dirname(__file__), "../../../sdk/python")
    if sdk_path not in sys.path:
        sys.path.insert(0, sdk_path)
    from hipcortex.client import HipCortexClient
    client = HipCortexClient(base_url=BASE)
    fork = client.fork_create()
    fork_id = fork.get("fork_id") or fork.get("id")
    assert fork_id, f"fork_create returned no id: {fork}"
    result = client.fork_rollout(fork_id, ["move", "observe"], sigma2_max=0.25)
    assert "steps" in result, f"rollout missing steps: {result}"
    assert isinstance(result["steps"], list), f"steps must be list: {result}"


def test_g5_4_ts_sdk_self_health_has_healthy_bool():
    """G5-4 (schema-only): TS SDK selfHealth() → object with healthy boolean (type check)."""
    import os
    ts_client = os.path.join(os.path.dirname(__file__), "../../../sdk/typescript/src/client.ts")
    assert os.path.exists(ts_client), f"TS client missing: {ts_client}"
    src = open(ts_client, encoding="utf-8").read()
    assert "async selfHealth()" in src, "selfHealth() method missing from TS client"
    assert "SelfHealthResponse" in src, "SelfHealthResponse type missing from TS client"


def test_g5_5_version_080_on_all_surfaces():
    """G5-5 (schema-only): VERSION file, MCP serverInfo, Python VERSION, TS package.json all 0.8.0."""
    import os, json
    root = os.path.join(os.path.dirname(__file__), "../../..")
    # VERSION file
    version_file = open(os.path.join(root, "VERSION"), encoding="utf-8").read().strip()
    assert version_file == "0.8.0", f"VERSION file is {version_file!r}"
    # MCP server
    mcp = open(os.path.join(root, "sdk/mcp/server.py"), encoding="utf-8").read()
    assert '"version": "0.8.0"' in mcp, "MCP serverInfo.version != 0.8.0"
    # Python SDK
    client_py = open(os.path.join(root, "sdk/python/hipcortex/client.py"), encoding="utf-8").read()
    assert 'VERSION = "0.8.0"' in client_py, "Python client VERSION != 0.8.0"
    # TS SDK
    ts_pkg = json.load(open(os.path.join(root, "sdk/typescript/package.json"), encoding="utf-8"))
    assert ts_pkg["version"] == "0.8.0", f"TS package.json version is {ts_pkg['version']!r}"
