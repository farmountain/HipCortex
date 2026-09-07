"""
Runtime proof that unknown sensors return {reachable: False, ok: False}.

Tests call execute_probe() directly with no server running — the early return
in the else-branch exits before any HTTP POST, so these are pure unit tests.
"""
import pytest
from hipcortex.runner import IntentRunner

# Runner pointed at a port nothing listens on — unknown sensor must never reach it
_RUNNER = IntentRunner(base_url="http://localhost:19999")


def _probe(sensor_path: str) -> dict:
    return _RUNNER.execute_probe("test-intent-id", "/some/target", sensor_path)


def _obs(r: dict) -> dict:
    """Return observation sub-dict from execute_probe result."""
    return r.get("observation", {})


def test_unknown_opaque_uri():
    r = _probe("xyz://unknown/sensor")
    assert r["ok"] is False
    assert _obs(r).get("reachable") is False
    assert "unknown_sensor" in _obs(r).get("error", "")


def test_empty_sensor_path():
    # empty string → coerced to "default" → hits else branch
    r = _probe("")
    assert r["ok"] is False
    assert _obs(r).get("reachable") is False


def test_explicit_default():
    r = _probe("default")
    assert r["ok"] is False
    assert _obs(r).get("reachable") is False


def test_ftp_scheme():
    r = _probe("ftp://server.example.com/file")
    assert r["ok"] is False
    assert _obs(r).get("reachable") is False


def test_numeric_sensor():
    r = _probe("12345")
    assert r["ok"] is False
    assert _obs(r).get("reachable") is False


def test_unknown_sensor_does_not_raise():
    # Must never raise — fail-silent is part of the probe contract
    try:
        r = _probe("completely_unknown_sensor_type")
        assert r["ok"] is False
    except Exception as exc:
        pytest.fail(f"execute_probe raised for unknown sensor: {exc}")


def test_known_filesystem_sensor_attempts_probe():
    # filesystem IS known — must NOT return unknown_sensor error
    r = _probe("filesystem")
    assert "unknown_sensor" not in _obs(r).get("error", ""), \
        "filesystem sensor must not be treated as unknown"
