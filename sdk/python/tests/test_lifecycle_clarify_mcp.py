"""AC-L7: MCP lifecycle handlers pass tiers_spent + cost_of_wrong_execution to REST body."""
import json
from unittest.mock import MagicMock, patch


def _load_ns():
    """Exec server.py with _req replaced by a MagicMock. Returns (namespace, mock)."""
    fake = MagicMock(return_value={"ok": True})
    src_path = "../../mcp/server.py"
    import os
    base = os.path.dirname(os.path.abspath(__file__))
    full = os.path.normpath(os.path.join(base, src_path))
    src = open(full, encoding="utf-8").read()
    ns: dict = {"__name__": "server", "json": json}
    exec(compile(src, full, "exec"), ns)  # noqa: S102 — test harness only
    ns["_req"] = fake
    return ns, fake


def test_check_progress_passes_tiers_spent_and_cost():
    ns, fake = _load_ns()
    ns["handle_check_progress"]({
        "success_factors": ["tests_passing"],
        "observations": ["started"],
        "iteration": 5,
        "max_iterations": 20,
        "tiers_spent": 2,
        "cost_of_wrong_execution": 0.8,
    })
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 2
    assert body["cost_of_wrong_execution"] == 0.8


def test_check_progress_defaults_tiers_spent_to_zero():
    ns, fake = _load_ns()
    ns["handle_check_progress"]({
        "success_factors": ["tests_passing"],
        "observations": ["o"],
        "iteration": 1,
        "max_iterations": 20,
    })
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 0
    assert body["cost_of_wrong_execution"] == 1.0


def test_plan_validation_passes_tiers_spent():
    ns, fake = _load_ns()
    ns["handle_plan_validation"]({"success_factors": ["tests_passing"], "tiers_spent": 1})
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 1


def test_plan_validation_defaults_tiers_spent_to_zero():
    ns, fake = _load_ns()
    ns["handle_plan_validation"]({"success_factors": ["tests_passing"]})
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 0


def test_should_exit_passes_tiers_spent():
    ns, fake = _load_ns()
    ns["handle_should_exit"]({
        "iteration": 5,
        "max_iterations": 20,
        "progress_ratio": 0.3,
        "surprise_signal": 0.1,
        "tiers_spent": 3,
    })
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 3


def test_should_exit_defaults_tiers_spent_to_zero():
    ns, fake = _load_ns()
    ns["handle_should_exit"]({"iteration": 1, "max_iterations": 20, "progress_ratio": 0.5})
    body = fake.call_args[0][2]
    assert body["tiers_spent"] == 0
