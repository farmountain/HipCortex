"""Tests for hipcortex doctor — offline skip + mocked health/probe.

Run: pytest sdk/python/tests/test_doctor.py -q
"""

from __future__ import annotations

import os
from unittest.mock import MagicMock, patch

import pytest


# ── helpers ──────────────────────────────────────────────────────────────────


def _health_resp(status=200, body=None):
    resp = MagicMock()
    resp.status_code = status
    resp.content = b"{}" if body is not None else b""
    payload = body if body is not None else {}
    resp.json.return_value = payload
    if status >= 400:
        import requests

        http_err = requests.HTTPError(response=resp)
        resp.raise_for_status.side_effect = http_err
    else:
        resp.raise_for_status.return_value = None
    return resp


# ── resolve / offline ────────────────────────────────────────────────────────


def test_resolve_url_default(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    from hipcortex.doctor import DEFAULT_URL, resolve_url

    assert resolve_url() == DEFAULT_URL
    assert resolve_url(None) == DEFAULT_URL


def test_resolve_url_env(monkeypatch):
    monkeypatch.setenv("HIPCORTEX_URL", "http://example:9999/")
    from hipcortex.doctor import resolve_url

    assert resolve_url() == "http://example:9999"


def test_resolve_url_explicit_wins(monkeypatch):
    monkeypatch.setenv("HIPCORTEX_URL", "http://env:1")
    from hipcortex.doctor import resolve_url

    assert resolve_url("http://arg:2/") == "http://arg:2"


def test_resolve_url_from_project_config(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    from hipcortex import config as cfg
    from hipcortex.config import ensure_project_config

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://proj:8080")
    from hipcortex.doctor import resolve_url

    assert resolve_url() == "http://proj:8080"


def test_is_offline(monkeypatch):
    from hipcortex.doctor import is_offline

    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    assert is_offline() is False
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    assert is_offline() is True
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "true")
    assert is_offline() is True
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "0")
    assert is_offline() is False


# ── offline path ─────────────────────────────────────────────────────────────


def test_doctor_offline_skips_network(monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)

    from hipcortex.doctor import format_report, run_doctor

    with patch("hipcortex.doctor.requests.Session") as sess_cls:
        report = run_doctor(probe=True)
        sess_cls.assert_not_called()

    assert report.ok is True
    statuses = {c.name: c.status for c in report.checks}
    assert statuses["health"] == "skip"
    assert statuses["version"] == "skip"
    assert statuses["probe"] == "skip"
    text = format_report(report)
    assert "SKIP" in text
    assert "HIPCORTEX_DOCTOR_OFFLINE" in text


def test_doctor_offline_without_probe_no_probe_check(monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    report = run_doctor(probe=False)
    names = [c.name for c in report.checks]
    assert "health" in names
    assert "probe" not in names


# ── mocked health ────────────────────────────────────────────────────────────


def test_doctor_health_ok_with_version(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )

    report = run_doctor(url="http://127.0.0.1:3030", session=sess)
    assert report.ok is True
    assert doctor_exit_code(report) == 0
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "ok"
    assert by_name["version"].status == "ok"
    assert "0.5.0" in by_name["version"].message
    sess.get.assert_called_once()
    assert sess.get.call_args[0][0] == "http://127.0.0.1:3030/health"


def test_doctor_health_ok_missing_version_warns(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, format_report, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(200, {"status": "ok"})

    report = run_doctor(session=sess)
    assert report.ok is True  # warn does not fail
    assert doctor_exit_code(report) == 0
    by_name = {c.name: c for c in report.checks}
    assert by_name["version"].status == "warn"
    assert "missing" in by_name["version"].message
    assert "PASS (with warnings)" in format_report(report)


def test_doctor_health_http_error(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(503, {"error": "down"})

    report = run_doctor(session=sess)
    assert report.ok is False
    assert doctor_exit_code(report) == 1
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "fail"
    assert by_name["version"].status == "skip"


def test_doctor_health_connection_error(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    import requests
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.side_effect = requests.ConnectionError("refused")

    report = run_doctor(session=sess, probe=True)
    assert report.ok is False
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "fail"
    assert by_name["probe"].status == "skip"


# ── probe ────────────────────────────────────────────────────────────────────


def test_doctor_probe_success(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    add_r = _health_resp(200, {"success": True, "record_id": "abc"})
    search_r = _health_resp(200, {"results": [{"score": 1.0, "record": {}}]})
    sess.post.side_effect = [add_r, search_r]

    report = run_doctor(probe=True, session=sess)
    assert report.ok is True
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    assert sess.post.call_count == 2
    add_url = sess.post.call_args_list[0][0][0]
    search_url = sess.post.call_args_list[1][0][0]
    assert add_url.endswith("/memory/add")
    assert search_url.endswith("/memory/search")


def test_doctor_probe_add_fails(monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    import requests
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    sess.post.side_effect = requests.Timeout("slow")

    report = run_doctor(probe=True, session=sess)
    assert report.ok is False
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "fail"
    assert "add" in by_name["probe"].message.lower()


# ── CLI wire ─────────────────────────────────────────────────────────────────


def test_cli_doctor_subcommand_registered():
    from hipcortex.cli import build_parser

    parser = build_parser()
    args = parser.parse_args(["doctor", "--probe", "--url", "http://x:1"])
    assert args.command == "doctor"
    assert args.probe is True
    assert args.url == "http://x:1"


def test_cmd_doctor_exits_zero_offline(monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.cli import cmd_doctor

    ns = MagicMock()
    ns.url = None
    ns.probe = False
    with pytest.raises(SystemExit) as ei:
        cmd_doctor(ns)
    assert ei.value.code == 0
