"""Tests for hipcortex doctor — offline skip + mocked health/probe + SKILL harness.

Run: pytest sdk/python/tests/test_doctor.py -q
"""

from __future__ import annotations

import os
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

# Minimal proactive SKILL harness language (MUST + live_beliefs, case-sensitive)
_GOOD_SKILL = (
    "# HipCortex Memory\n\n"
    "MUST: Before questions call get_live_beliefs first.\n"
    "Harness observations = live_beliefs (merged substrate).\n"
)
_BAD_SKILL_NO_MUST = "# skill\nlive_beliefs only, no marker.\n"
_BAD_SKILL_NO_LIVE = "# skill\nMUST call search first.\n"
_BAD_SKILL_EMPTY = "# empty skill without harness language\n"


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


def _write_skill(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def _skill_kw(tmp_path: Path, *, installed: str | None = "missing") -> dict:
    """Isolate skill checks from real ~/.claude (deterministic unit tests).

    package_skill → real package template (unit of truth, always present in tree).
    installed → missing path (warn) or written good/bad skill under tmp_path.
    """
    from hipcortex.doctor import package_skill_path

    pkg = package_skill_path()
    if installed == "missing":
        inst = tmp_path / "no_skill" / "SKILL.md"
    elif installed == "good":
        inst = _write_skill(tmp_path / "claude" / "SKILL.md", _GOOD_SKILL)
    elif installed == "bad_must":
        inst = _write_skill(tmp_path / "claude" / "SKILL.md", _BAD_SKILL_NO_MUST)
    elif installed == "bad_live":
        inst = _write_skill(tmp_path / "claude" / "SKILL.md", _BAD_SKILL_NO_LIVE)
    else:
        inst = Path(installed)  # type: ignore[arg-type]
    return {"package_skill": pkg, "installed_skill": inst}


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


def test_doctor_offline_skips_network(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)

    from hipcortex.doctor import format_report, run_doctor

    with patch("hipcortex.doctor.requests.Session") as sess_cls:
        report = run_doctor(probe=True, **_skill_kw(tmp_path))
        sess_cls.assert_not_called()

    assert report.ok is True
    statuses = {c.name: c.status for c in report.checks}
    assert statuses["health"] == "skip"
    assert statuses["version"] == "skip"
    assert statuses["probe"] == "skip"
    assert statuses["skill_package"] == "ok"
    assert statuses["skill_installed"] == "warn"  # missing under tmp
    text = format_report(report)
    assert "SKIP" in text or "PASS" in text
    assert "HIPCORTEX_DOCTOR_OFFLINE" in text


def test_doctor_offline_without_probe_no_probe_check(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    report = run_doctor(probe=False, **_skill_kw(tmp_path))
    names = [c.name for c in report.checks]
    assert "health" in names
    assert "skill_package" in names
    assert "skill_installed" in names
    assert "probe" not in names


# ── proactive SKILL harness ──────────────────────────────────────────────────


def test_skill_package_template_has_harness_markers():
    """Package install/SKILL.md is unit of truth — MUST + live_beliefs present."""
    from hipcortex.doctor import package_skill_path, skill_missing_markers

    path = package_skill_path()
    assert path.is_file(), f"package SKILL missing: {path}"
    text = path.read_text(encoding="utf-8")
    assert skill_missing_markers(text) == []
    assert "MUST" in text
    assert "live_beliefs" in text


def test_skill_missing_markers_case_sensitive():
    from hipcortex.doctor import skill_missing_markers

    assert skill_missing_markers("must live_beliefs") == ["MUST"]
    assert skill_missing_markers("MUST LIVE_BELIEFS") == ["live_beliefs"]
    assert skill_missing_markers(_GOOD_SKILL) == []
    assert set(skill_missing_markers(_BAD_SKILL_EMPTY)) == {"MUST", "live_beliefs"}


def test_doctor_skill_installed_missing_warns(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import doctor_exit_code, run_doctor

    report = run_doctor(**_skill_kw(tmp_path, installed="missing"))
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_package"].status == "ok"
    assert by_name["skill_installed"].status == "warn"
    assert "not found" in by_name["skill_installed"].message
    assert report.ok is True  # warn does not fail
    assert doctor_exit_code(report) == 0


def test_doctor_skill_installed_good_ok(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    report = run_doctor(**_skill_kw(tmp_path, installed="good"))
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_package"].status == "ok"
    assert by_name["skill_installed"].status == "ok"
    assert report.ok is True


def test_doctor_skill_installed_missing_must_fails(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import doctor_exit_code, run_doctor

    report = run_doctor(**_skill_kw(tmp_path, installed="bad_must"))
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_installed"].status == "fail"
    assert "MUST" in by_name["skill_installed"].message
    assert report.ok is False
    assert doctor_exit_code(report) == 1


def test_doctor_skill_installed_missing_live_beliefs_fails(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    report = run_doctor(**_skill_kw(tmp_path, installed="bad_live"))
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_installed"].status == "fail"
    assert "live_beliefs" in by_name["skill_installed"].message
    assert report.ok is False


def test_doctor_skill_package_missing_fails(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import doctor_exit_code, run_doctor

    missing_pkg = tmp_path / "absent" / "SKILL.md"
    good_inst = _write_skill(tmp_path / "claude" / "SKILL.md", _GOOD_SKILL)
    report = run_doctor(package_skill=missing_pkg, installed_skill=good_inst)
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_package"].status == "fail"
    assert by_name["skill_installed"].status == "ok"
    assert report.ok is False
    assert doctor_exit_code(report) == 1


def test_doctor_skill_package_incomplete_fails(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    bad_pkg = _write_skill(tmp_path / "pkg" / "SKILL.md", _BAD_SKILL_EMPTY)
    report = run_doctor(
        package_skill=bad_pkg,
        installed_skill=tmp_path / "no" / "SKILL.md",
    )
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_package"].status == "fail"
    assert "MUST" in by_name["skill_package"].message
    assert "live_beliefs" in by_name["skill_package"].message
    assert report.ok is False


def test_doctor_skill_via_monkeypatch_home(tmp_path, monkeypatch):
    """installed_skill_path uses Path.home() — monkeypatch home for realism."""
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    home = tmp_path / "home"
    skill = home / ".claude" / "skills" / "hipcortex" / "SKILL.md"
    _write_skill(skill, _GOOD_SKILL)
    monkeypatch.setattr(Path, "home", lambda: home)

    from hipcortex.doctor import installed_skill_path, package_skill_path, run_doctor

    assert installed_skill_path() == skill
    # only override package; installed resolves via home
    report = run_doctor(package_skill=package_skill_path())
    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_installed"].status == "ok"


def test_doctor_offline_still_runs_skill_checks(tmp_path, monkeypatch):
    """Brief: offline mode still runs SKILL file check (no network)."""
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.doctor import run_doctor

    with patch("hipcortex.doctor.requests.Session") as sess_cls:
        report = run_doctor(probe=True, **_skill_kw(tmp_path, installed="good"))
        sess_cls.assert_not_called()

    by_name = {c.name: c for c in report.checks}
    assert by_name["skill_package"].status == "ok"
    assert by_name["skill_installed"].status == "ok"
    assert by_name["health"].status == "skip"


# ── mocked health ────────────────────────────────────────────────────────────


def test_doctor_health_ok_with_version(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )

    report = run_doctor(
        url="http://127.0.0.1:3030", session=sess, **_skill_kw(tmp_path)
    )
    assert report.ok is True
    assert doctor_exit_code(report) == 0
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "ok"
    assert by_name["version"].status == "ok"
    assert "0.5.0" in by_name["version"].message
    assert by_name["skill_package"].status == "ok"
    sess.get.assert_called_once()
    assert sess.get.call_args[0][0] == "http://127.0.0.1:3030/health"


def test_doctor_health_ok_missing_version_warns(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, format_report, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(200, {"status": "ok"})

    report = run_doctor(session=sess, **_skill_kw(tmp_path))
    assert report.ok is True  # warn does not fail
    assert doctor_exit_code(report) == 0
    by_name = {c.name: c for c in report.checks}
    assert by_name["version"].status == "warn"
    assert "missing" in by_name["version"].message
    assert "PASS (with warnings)" in format_report(report)


def test_doctor_health_http_error(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(503, {"error": "down"})

    report = run_doctor(session=sess, **_skill_kw(tmp_path))
    assert report.ok is False
    assert doctor_exit_code(report) == 1
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "fail"
    assert by_name["version"].status == "skip"
    # skill checks still ran before health
    assert by_name["skill_package"].status == "ok"


def test_doctor_health_connection_error(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    import requests
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.side_effect = requests.ConnectionError("refused")

    report = run_doctor(session=sess, probe=True, **_skill_kw(tmp_path))
    assert report.ok is False
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "fail"
    assert by_name["probe"].status == "skip"


# ── probe ────────────────────────────────────────────────────────────────────


def test_is_local_doctor_url():
    from hipcortex.doctor import is_local_doctor_url

    assert is_local_doctor_url("http://localhost:3030") is True
    assert is_local_doctor_url("http://127.0.0.1:3030") is True
    assert is_local_doctor_url("http://[::1]:3030") is True
    assert is_local_doctor_url("http://0.0.0.0:3030") is True
    assert is_local_doctor_url("https://example.com") is False
    assert is_local_doctor_url("https://hipcortex.fly.dev") is False


def test_doctor_probe_empty_search_fails(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    add_r = _health_resp(200, {"success": True, "record_id": "abc"})
    search_r = _health_resp(200, {"results": []})
    sess.post.side_effect = [add_r, search_r]

    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is False
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "fail"
    assert "0 result" in by_name["probe"].message.lower() or "no hit" in by_name[
        "probe"
    ].message.lower() or "no match" in by_name["probe"].message.lower()


def test_doctor_probe_success(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    add_r = _health_resp(200, {"success": True, "record_id": "abc"})

    def _search_side_effect(url, json=None, timeout=None, **kwargs):
        # Return hit matching the unique target from the add payload
        add_payload = sess.post.call_args_list[0]
        add_json = add_payload.kwargs.get("json") or add_payload[1].get("json")
        target = add_json["target"]
        return _health_resp(
            200,
            {
                "results": [
                    {
                        "score": 1.0,
                        "record": {"target": target, "action": "probe"},
                    }
                ]
            },
        )

    def _post_side_effect(url, json=None, timeout=None, **kwargs):
        if url.endswith("/memory/add"):
            return add_r
        return _search_side_effect(url, json=json, timeout=timeout)

    sess.post.side_effect = _post_side_effect

    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is True
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    assert sess.post.call_count == 2
    add_url = sess.post.call_args_list[0][0][0]
    search_url = sess.post.call_args_list[1][0][0]
    assert add_url.endswith("/memory/add")
    assert search_url.endswith("/memory/search")
    add_json = sess.post.call_args_list[0].kwargs.get("json") or sess.post.call_args_list[0][1].get(
        "json"
    )
    assert add_json["target"].startswith("doctor-roundtrip-")


def test_doctor_probe_remote_skipped_by_default(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    monkeypatch.delenv("HIPCORTEX_DOCTOR_PROBE_REMOTE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )

    report = run_doctor(
        url="https://hipcortex.fly.dev",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    by_name = {c.name: c for c in report.checks}
    assert by_name["health"].status == "ok"
    assert by_name["probe"].status == "skip"
    assert "remote" in by_name["probe"].message.lower()
    assert "HIPCORTEX_DOCTOR_PROBE_REMOTE" in by_name["probe"].message
    sess.post.assert_not_called()
    # remote skip is not a hard fail
    assert report.ok is True


def test_doctor_probe_remote_allowed_with_env(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    monkeypatch.setenv("HIPCORTEX_DOCTOR_PROBE_REMOTE", "1")
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    add_r = _health_resp(200, {"success": True, "record_id": "abc"})

    def _post_side_effect(url, json=None, timeout=None, **kwargs):
        if url.endswith("/memory/add"):
            return add_r
        target = json["query"] if json and "query" in json else "doctor-roundtrip"
        # Prefer target from prior add
        add_json = sess.post.call_args_list[0].kwargs.get("json") or sess.post.call_args_list[0][
            1
        ].get("json")
        target = add_json["target"]
        return _health_resp(
            200, {"results": [{"score": 1.0, "record": {"target": target}}]}
        )

    sess.post.side_effect = _post_side_effect

    report = run_doctor(
        url="https://hipcortex.fly.dev",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    assert "remote" in by_name["probe"].message.lower()
    assert sess.post.call_count == 2
    assert report.ok is True


def test_doctor_probe_add_fails(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    import requests
    from hipcortex.doctor import run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    sess.post.side_effect = requests.Timeout("slow")

    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is False
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "fail"
    assert "add" in by_name["probe"].message.lower()


# ── probe cleanup ─────────────────────────────────────────────────────────────


def _probe_success_session(record_id=None, add_body=None, delete_error=None):
    """Session mock: healthy + add + search hit; optional DELETE fail."""
    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "0.5.0", "status": "ok"}
    )
    if add_body is None:
        body = {"success": True}
        if record_id is not None:
            body["record_id"] = record_id
        add_body = body
    add_r = _health_resp(200, add_body)

    def _post_side_effect(url, json=None, timeout=None, **kwargs):
        if url.endswith("/memory/add"):
            return add_r
        add_json = sess.post.call_args_list[0].kwargs.get("json") or sess.post.call_args_list[0][
            1
        ].get("json")
        target = add_json["target"]
        return _health_resp(
            200,
            {
                "results": [
                    {"score": 1.0, "record": {"target": target, "action": "probe"}}
                ]
            },
        )

    sess.post.side_effect = _post_side_effect
    if delete_error is not None:
        sess.delete.side_effect = delete_error
    else:
        sess.delete.return_value = _health_resp(200, {"success": True})
    return sess


def test_doctor_probe_cleanup_deletes_record(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = _probe_success_session(record_id="abc")
    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is True
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    assert by_name["probe"].detail["cleanup"] == "ok"
    sess.delete.assert_called_once()
    del_url = sess.delete.call_args[0][0]
    assert del_url.endswith("/memory/abc")


def test_doctor_probe_cleanup_fail_still_ok(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    import requests
    from hipcortex.doctor import run_doctor

    sess = _probe_success_session(
        record_id="abc", delete_error=requests.ConnectionError("gone")
    )
    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is True
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    cleanup = by_name["probe"].detail["cleanup"]
    assert cleanup.startswith("failed:")
    sess.delete.assert_called_once()


def test_doctor_probe_cleanup_skipped_no_id(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import run_doctor

    sess = _probe_success_session(add_body={"success": True})
    report = run_doctor(
        url="http://127.0.0.1:3030",
        probe=True,
        session=sess,
        **_skill_kw(tmp_path),
    )
    assert report.ok is True
    by_name = {c.name: c for c in report.checks}
    assert by_name["probe"].status == "ok"
    assert by_name["probe"].detail["cleanup"] == "skipped_no_id"
    sess.delete.assert_not_called()


# ── CLI wire ─────────────────────────────────────────────────────────────────


def test_cli_doctor_subcommand_registered():
    from hipcortex.cli import build_parser

    parser = build_parser()
    args = parser.parse_args(["doctor", "--probe", "--url", "http://x:1"])
    assert args.command == "doctor"
    assert args.probe is True
    assert args.url == "http://x:1"


def test_cmd_doctor_exits_zero_offline(monkeypatch):
    """CLI offline: exit 0 when package SKILL ok (installed missing → warn only)."""
    monkeypatch.setenv("HIPCORTEX_DOCTOR_OFFLINE", "1")
    from hipcortex.cli import cmd_doctor
    from hipcortex.doctor import package_skill_path

    # Isolate installed skill so a broken real ~/.claude skill cannot fail CI
    real_run = None
    import hipcortex.doctor as doctor_mod

    real_run = doctor_mod.run_doctor

    def _run(*args, **kwargs):
        kwargs.setdefault("package_skill", package_skill_path())
        kwargs.setdefault(
            "installed_skill",
            Path("/nonexistent/hipcortex-doctor-test/SKILL.md"),
        )
        return real_run(*args, **kwargs)

    monkeypatch.setattr(doctor_mod, "run_doctor", _run)

    ns = MagicMock()
    ns.url = None
    ns.probe = False
    with pytest.raises(SystemExit) as ei:
        cmd_doctor(ns)
    assert ei.value.code == 0


# ── version_policy_ok ────────────────────────────────────────────────────────


def test_version_policy_major_minor_patch_ok():
    from hipcortex.doctor import version_policy_ok

    ok, msg = version_policy_ok("0.5.1", "0.5.0", "major_minor")
    assert ok is True
    assert "major_minor" in msg


def test_version_policy_major_minor_mismatch():
    from hipcortex.doctor import version_policy_ok

    ok, msg = version_policy_ok("0.6.0", "0.5.0", "major_minor")
    assert ok is False
    assert "mismatch" in msg


def test_version_policy_exact():
    from hipcortex.doctor import version_policy_ok

    ok, _ = version_policy_ok("0.5.0", "0.5.0", "exact")
    assert ok is True
    ok2, msg2 = version_policy_ok("0.5.1", "0.5.0", "exact")
    assert ok2 is False
    assert "exact" in msg2


def test_version_policy_any_healthy_none_version():
    from hipcortex.doctor import version_policy_ok

    ok, msg = version_policy_ok(None, "0.5.0", "any_healthy")
    assert ok is True
    assert "any_healthy" in msg


def test_version_policy_missing_server_not_ok_for_strict():
    from hipcortex.doctor import version_policy_ok

    ok, msg = version_policy_ok(None, "0.5.0", "major_minor")
    assert ok is False
    assert "missing" in msg


def test_version_policy_unknown_falls_back_to_major_minor():
    from hipcortex.doctor import version_policy_ok

    ok, _ = version_policy_ok("0.5.9", "0.5.0", "weird_policy")
    assert ok is True
    ok2, _ = version_policy_ok("1.0.0", "0.5.0", "weird_policy")
    assert ok2 is False


def test_doctor_version_mismatch_warns(tmp_path, monkeypatch):
    """major_minor mismatch → warn, not fail."""
    monkeypatch.delenv("HIPCORTEX_DOCTOR_OFFLINE", raising=False)
    from hipcortex.doctor import doctor_exit_code, run_doctor

    sess = MagicMock()
    sess.get.return_value = _health_resp(
        200, {"service": "hipcortex", "version": "9.9.0", "status": "ok"}
    )

    report = run_doctor(session=sess, **_skill_kw(tmp_path))
    assert report.ok is True  # warn does not fail
    assert doctor_exit_code(report) == 0
    by_name = {c.name: c for c in report.checks}
    assert by_name["version"].status == "warn"
    assert "mismatch" in by_name["version"].message
