"""Tests for hipcortex.daemon — lock/pid/health helpers (mocked, no real binary).

Run: pytest sdk/python/tests/test_daemon.py -q
"""

from __future__ import annotations

import io
import json
import os
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest


@pytest.fixture
def daemon_paths(tmp_path, monkeypatch):
    """Point daemon install paths at tmp_path."""
    from hipcortex import daemon as d

    install = tmp_path / ".hipcortex"
    install.mkdir()
    monkeypatch.setattr(d, "INSTALL_DIR", install)
    monkeypatch.setattr(d, "PID_FILE", install / "hipcortex.pid")
    monkeypatch.setattr(d, "LOCK_FILE", install / "server.lock")
    monkeypatch.setattr(d, "DEFAULT_DATA_DIR", install / "data")
    return d


# ── constants ────────────────────────────────────────────────────────────────


def test_default_data_dir_is_under_install():
    from hipcortex.daemon import DEFAULT_DATA_DIR, INSTALL_DIR

    assert DEFAULT_DATA_DIR == INSTALL_DIR / "data"
    assert INSTALL_DIR.name == ".hipcortex"


# ── pid helpers ──────────────────────────────────────────────────────────────


def test_write_read_clear_pid(daemon_paths):
    d = daemon_paths
    assert d.read_pid() is None
    d.write_pid(4242)
    assert d.read_pid() == 4242
    assert d.PID_FILE.exists()
    d.clear_pid()
    assert d.read_pid() is None
    assert not d.PID_FILE.exists()


def test_read_pid_corrupt(daemon_paths):
    d = daemon_paths
    d.PID_FILE.write_text("not-a-pid", encoding="utf-8")
    assert d.read_pid() is None


# ── lock helpers ─────────────────────────────────────────────────────────────


def test_acquire_release_lock(daemon_paths):
    d = daemon_paths
    assert d.acquire_lock(owner_pid=111) is True
    assert d.LOCK_FILE.exists()
    assert d.LOCK_FILE.read_text(encoding="utf-8").strip() == "111"
    # second acquire while alive holder fails
    with patch.object(d, "is_pid_alive", return_value=True):
        assert d.acquire_lock(owner_pid=222) is False
    d.release_lock()
    assert not d.LOCK_FILE.exists()
    assert d.acquire_lock(owner_pid=222) is True


def test_acquire_lock_stale_cleanup(daemon_paths):
    d = daemon_paths
    d.LOCK_FILE.write_text("99999\n", encoding="utf-8")
    with patch.object(d, "is_pid_alive", return_value=False):
        assert d.acquire_lock(owner_pid=7) is True
    assert d.LOCK_FILE.read_text(encoding="utf-8").strip() == "7"


# ── port / health ────────────────────────────────────────────────────────────


def test_is_port_open_closed(daemon_paths):
    d = daemon_paths
    # high port unlikely open
    assert d.is_port_open("127.0.0.1", 1) is False


def test_health_check_ok(daemon_paths):
    d = daemon_paths
    payload = {"service": "hipcortex", "status": "ok", "version": "0.5.0"}
    body = json.dumps(payload).encode()

    mock_resp = MagicMock()
    mock_resp.status = 200
    mock_resp.read.return_value = body
    mock_resp.__enter__ = lambda s: s
    mock_resp.__exit__ = MagicMock(return_value=False)

    with patch("urllib.request.urlopen", return_value=mock_resp):
        data = d.health_check("http://127.0.0.1:3030")
    assert data == payload


def test_health_check_down(daemon_paths):
    d = daemon_paths
    with patch("urllib.request.urlopen", side_effect=OSError("down")):
        assert d.health_check("http://127.0.0.1:3030") is None


# ── start_server (mocked spawn) ──────────────────────────────────────────────


def test_start_server_already_running(daemon_paths, tmp_path):
    d = daemon_paths
    binary = tmp_path / "hipcortex-server"
    binary.write_text("x")

    with patch.object(
        d,
        "health_check",
        return_value={"service": "hipcortex", "status": "ok", "version": "1"},
    ):
        with pytest.raises(d.AlreadyRunning):
            d.start_server(binary, port=3030)


def test_start_server_port_in_use(daemon_paths, tmp_path):
    d = daemon_paths
    binary = tmp_path / "hipcortex-server"
    binary.write_text("x")

    with patch.object(d, "health_check", return_value=None):
        with patch.object(d, "is_port_open", return_value=True):
            with pytest.raises(d.PortInUse):
                d.start_server(binary, port=3030)


def test_start_server_success(daemon_paths, tmp_path):
    d = daemon_paths
    binary = tmp_path / "hipcortex-server"
    binary.write_text("x")
    data_dir = tmp_path / "data"

    proc = MagicMock()
    proc.pid = 5555

    health = {"service": "hipcortex", "status": "ok", "version": "0.5.0"}

    with patch.object(d, "health_check", return_value=None):
        with patch.object(d, "is_port_open", return_value=False):
            with patch.object(d, "is_pid_alive", return_value=False):
                with patch.object(d.subprocess, "Popen", return_value=proc) as popen:
                    with patch.object(d, "wait_for_health", return_value=health):
                        result = d.start_server(binary, port=4040, data_dir=data_dir)

    assert result["pid"] == 5555
    assert result["port"] == 4040
    assert result["ready"] is True
    assert result["health"]["version"] == "0.5.0"
    assert d.read_pid() == 5555
    assert d.LOCK_FILE.exists()
    popen.assert_called_once()
    env = popen.call_args.kwargs["env"]
    assert env["PORT"] == "4040"
    assert env["DATA_DIR"] == str(data_dir)


def test_start_server_lock_held(daemon_paths, tmp_path):
    d = daemon_paths
    binary = tmp_path / "hipcortex-server"
    binary.write_text("x")
    d.LOCK_FILE.write_text("1234\n", encoding="utf-8")

    with patch.object(d, "health_check", return_value=None):
        with patch.object(d, "is_port_open", return_value=False):
            with patch.object(d, "is_pid_alive", return_value=True):
                with pytest.raises(d.LockError):
                    d.start_server(binary, port=3030)


# ── stop_server ──────────────────────────────────────────────────────────────


def test_stop_server_via_pid(daemon_paths):
    d = daemon_paths
    d.write_pid(7777)
    d.LOCK_FILE.write_text("7777\n", encoding="utf-8")

    with patch.object(d, "is_pid_alive", side_effect=[True, False]):
        with patch.object(d, "_terminate_pid") as term:
            result = d.stop_server(port=3030)

    assert result["stopped"] is True
    assert result["method"] == "pid"
    assert result["pid"] == 7777
    term.assert_called()
    assert d.read_pid() is None
    assert not d.LOCK_FILE.exists()


def test_stop_server_none(daemon_paths):
    d = daemon_paths
    with patch.object(d, "is_port_open", return_value=False):
        with patch.object(d, "_pids_on_port", return_value=[]):
            result = d.stop_server(port=3030)
    assert result["stopped"] is False
    assert result["method"] == "none"


def test_stop_server_port_fallback(daemon_paths):
    d = daemon_paths
    # no pid file / no lock — only kill port PIDs when health proves hipcortex
    with patch.object(
        d, "health_check", return_value={"service": "hipcortex", "status": "ok"}
    ):
        with patch.object(d, "_pids_on_port", return_value=[8888]):
            with patch.object(d, "is_pid_alive", side_effect=[True, False, False]):
                with patch.object(d, "_terminate_pid") as term:
                    result = d.stop_server(port=3030)

    assert result["stopped"] is True
    assert result["method"] == "port"
    assert result["pid"] == 8888
    term.assert_called()


def test_netstat_line_matches_listen_port(daemon_paths):
    d = daemon_paths
    assert d._netstat_line_matches_listen_port(
        "  TCP    127.0.0.1:3030         0.0.0.0:0              LISTENING       1234",
        3030,
    )
    assert d._netstat_line_matches_listen_port(
        "  TCP    0.0.0.0:3030           0.0.0.0:0              LISTENING       99",
        3030,
    )
    assert d._netstat_line_matches_listen_port(
        "  TCP    [::]:3030              [::]:0                 LISTENING       1",
        3030,
    )
    # must not match superstring ports
    assert not d._netstat_line_matches_listen_port(
        "  TCP    127.0.0.1:30301        0.0.0.0:0              LISTENING       1",
        3030,
    )
    assert not d._netstat_line_matches_listen_port(
        "  TCP    127.0.0.1:13030        0.0.0.0:0              LISTENING       1",
        3030,
    )
    # non-LISTENING
    assert not d._netstat_line_matches_listen_port(
        "  TCP    127.0.0.1:3030         127.0.0.1:50000        ESTABLISHED     1",
        3030,
    )


def test_windows_pid_alive_token(daemon_paths):
    d = daemon_paths
    # whole-token: 123 must not match a 1234-only line
    assert d._tasklist_output_matches_pid(
        "python.exe                   123 Console                    1     10,000 K",
        123,
    )
    assert not d._tasklist_output_matches_pid(
        "python.exe                  1234 Console                    1     10,000 K",
        123,
    )
    assert not d._tasklist_output_matches_pid("INFO: No tasks are running which match the specified criteria.", 123)
    assert not d._tasklist_output_matches_pid("", 123)


def test_start_server_foreign_ok_health_is_port_in_use(daemon_paths, tmp_path):
    d = daemon_paths
    binary = tmp_path / "hipcortex-server"
    binary.write_text("x")

    # Foreign HTTP 200 with status=ok but no service=hipcortex must not be AlreadyRunning
    with patch.object(d, "health_check", return_value={"status": "ok"}):
        with patch.object(d, "is_port_open", return_value=True):
            with pytest.raises(d.PortInUse):
                d.start_server(binary, port=3030)

    with patch.object(d, "health_check", return_value={"status": "ok", "version": "1.0"}):
        with patch.object(d, "is_port_open", return_value=True):
            with pytest.raises(d.PortInUse):
                d.start_server(binary, port=3030)


def test_stop_server_no_kill_without_evidence(daemon_paths):
    d = daemon_paths
    # no pid, no lock, health None, but port has listeners → do not kill
    with patch.object(d, "health_check", return_value=None):
        with patch.object(d, "_pids_on_port", return_value=[9999, 8888]):
            with patch.object(d, "_terminate_pid") as term:
                with patch.object(d, "_force_kill_pid") as force:
                    result = d.stop_server(port=3030)

    assert result["stopped"] is False
    assert result["method"] == "none"
    term.assert_not_called()
    force.assert_not_called()


def test_stop_server_via_lock_pid(daemon_paths):
    d = daemon_paths
    d.LOCK_FILE.write_text("6666\n", encoding="utf-8")

    with patch.object(d, "is_pid_alive", side_effect=[True, False]):
        with patch.object(d, "_terminate_pid") as term:
            result = d.stop_server(port=3030)

    assert result["stopped"] is True
    assert result["method"] == "lock"
    assert result["pid"] == 6666
    term.assert_called()
    assert not d.LOCK_FILE.exists()


# ── resolve_data_dir ─────────────────────────────────────────────────────────


def test_resolve_data_dir_default(daemon_paths, monkeypatch):
    d = daemon_paths
    monkeypatch.delenv("DATA_DIR", raising=False)
    path = d.resolve_data_dir()
    assert path == d.DEFAULT_DATA_DIR
    assert path.exists()


def test_resolve_data_dir_env(daemon_paths, tmp_path, monkeypatch):
    d = daemon_paths
    custom = tmp_path / "custom-data"
    monkeypatch.setenv("DATA_DIR", str(custom))
    path = d.resolve_data_dir()
    assert path == custom
    assert path.exists()
