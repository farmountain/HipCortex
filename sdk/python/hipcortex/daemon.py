"""Shared daemon protocol for HipCortex local server lifecycle.

Single attach model: one supervised process, one lock, one default data dir
under ``~/.hipcortex/``. Clients attach via health; only this module spawns.
"""

from __future__ import annotations

import json
import os
import platform
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Dict, Optional, Union

# ─── Paths (unified install root) ────────────────────────────────────────────

INSTALL_DIR: Path = Path.home() / ".hipcortex"
PID_FILE: Path = INSTALL_DIR / "hipcortex.pid"
LOCK_FILE: Path = INSTALL_DIR / "server.lock"
DEFAULT_DATA_DIR: Path = INSTALL_DIR / "data"

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 3030
DEFAULT_URL = f"http://{DEFAULT_HOST}:{DEFAULT_PORT}"

PathLike = Union[str, Path]


class DaemonError(Exception):
    """Base error for daemon lifecycle failures."""


class AlreadyRunning(DaemonError):
    """HipCortex already healthy on the target port."""


class PortInUse(DaemonError):
    """Port open but not a healthy HipCortex server."""


class LockError(DaemonError):
    """Could not acquire exclusive server.lock."""


# ─── Helpers ─────────────────────────────────────────────────────────────────


def ensure_install_dir() -> Path:
    INSTALL_DIR.mkdir(parents=True, exist_ok=True)
    return INSTALL_DIR


def resolve_data_dir(data_dir: Optional[PathLike] = None) -> Path:
    """Default always ``~/.hipcortex/data`` (not vscode path)."""
    if data_dir is not None:
        path = Path(data_dir)
    else:
        env = os.environ.get("DATA_DIR")
        path = Path(env) if env else DEFAULT_DATA_DIR
    path.mkdir(parents=True, exist_ok=True)
    return path


def health_check(url: str, timeout: float = 2.0) -> Optional[Dict[str, Any]]:
    """GET ``{url}/health`` (or url if already ends with /health). Return dict or None."""
    base = url.rstrip("/")
    health_url = base if base.endswith("/health") else f"{base}/health"
    try:
        req = urllib.request.Request(health_url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            if resp.status != 200:
                return None
            raw = resp.read().decode("utf-8", errors="replace")
            try:
                data = json.loads(raw)
            except json.JSONDecodeError:
                return {"raw": raw, "status": "ok"}
            if isinstance(data, dict):
                return data
            return {"raw": data, "status": "ok"}
    except (urllib.error.URLError, TimeoutError, OSError, ValueError):
        return None


def is_port_open(host: str, port: int, timeout: float = 0.5) -> bool:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(timeout)
    try:
        return sock.connect_ex((host, int(port))) == 0
    except OSError:
        return False
    finally:
        try:
            sock.close()
        except OSError:
            pass


def is_pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    # Windows: os.kill(pid, 0) is not a no-op (can terminate). Use tasklist.
    if platform.system() == "Windows":
        return _windows_pid_alive(pid)
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        # Exists but we cannot signal it — treat as alive.
        return True
    except OSError:
        return False


def _windows_pid_alive(pid: int) -> bool:
    try:
        # tasklist is ubiquitous; avoid psutil dependency.
        result = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}", "/NH"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        out = (result.stdout or "").strip()
        if not out or "No tasks" in out or "INFO:" in out:
            return False
        return str(pid) in out
    except (OSError, subprocess.SubprocessError):
        return False


# ─── PID file ────────────────────────────────────────────────────────────────


def read_pid() -> Optional[int]:
    if not PID_FILE.exists():
        return None
    try:
        text = PID_FILE.read_text(encoding="utf-8").strip()
        if not text:
            return None
        return int(text)
    except (OSError, ValueError):
        return None


def write_pid(pid: int) -> None:
    ensure_install_dir()
    PID_FILE.write_text(str(int(pid)), encoding="utf-8")


def clear_pid() -> None:
    try:
        PID_FILE.unlink(missing_ok=True)
    except OSError:
        pass


# ─── Lock file (exclusive create + stale cleanup) ────────────────────────────


def _read_lock_pid() -> Optional[int]:
    if not LOCK_FILE.exists():
        return None
    try:
        text = LOCK_FILE.read_text(encoding="utf-8").strip()
        if not text:
            return None
        # lock content: plain pid or "pid=<n>"
        if text.startswith("pid="):
            text = text.split("=", 1)[1].strip()
        return int(text.split()[0])
    except (OSError, ValueError, IndexError):
        return None


def acquire_lock(owner_pid: Optional[int] = None) -> bool:
    """Exclusive ``server.lock`` via O_CREAT|O_EXCL. Stale cleanup if holder dead.

    Returns True if this process holds the lock after the call.
    """
    ensure_install_dir()
    pid = int(owner_pid if owner_pid is not None else os.getpid())

    if LOCK_FILE.exists():
        holder = _read_lock_pid()
        if holder is not None and is_pid_alive(holder):
            return False
        # Stale lock (dead pid or unreadable)
        try:
            LOCK_FILE.unlink(missing_ok=True)
        except OSError:
            return False

    flags = os.O_CREAT | os.O_EXCL | os.O_WRONLY
    try:
        fd = os.open(str(LOCK_FILE), flags, 0o644)
    except FileExistsError:
        return False
    except OSError:
        return False
    try:
        os.write(fd, f"{pid}\n".encode("utf-8"))
    finally:
        os.close(fd)
    return True


def release_lock() -> None:
    try:
        LOCK_FILE.unlink(missing_ok=True)
    except OSError:
        pass


# ─── Process control ─────────────────────────────────────────────────────────


def _terminate_pid(pid: int) -> None:
    if platform.system() == "Windows":
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/T"],
            capture_output=True,
            text=True,
        )
        return
    try:
        os.kill(pid, signal.SIGTERM)
    except OSError:
        pass


def _force_kill_pid(pid: int) -> None:
    if platform.system() == "Windows":
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/T", "/F"],
            capture_output=True,
            text=True,
        )
        return
    try:
        os.kill(pid, signal.SIGKILL)
    except OSError:
        pass


def _pids_on_port(port: int) -> list[int]:
    """Best-effort discover PIDs listening on port."""
    pids: list[int] = []
    try:
        if platform.system() == "Windows":
            result = subprocess.run(
                ["netstat", "-ano"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            for line in (result.stdout or "").splitlines():
                if f":{port}" not in line or "LISTENING" not in line.upper():
                    continue
                parts = line.strip().split()
                if not parts:
                    continue
                try:
                    pids.append(int(parts[-1]))
                except ValueError:
                    continue
        else:
            result = subprocess.run(
                ["lsof", "-ti", f":{port}"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            for token in (result.stdout or "").strip().split():
                try:
                    pids.append(int(token))
                except ValueError:
                    continue
    except (OSError, subprocess.SubprocessError):
        return []
    # unique preserve order
    seen = set()
    out: list[int] = []
    for p in pids:
        if p not in seen and p > 0:
            seen.add(p)
            out.append(p)
    return out


def wait_for_health(
    url: str,
    *,
    timeout: float = 10.0,
    interval: float = 0.5,
) -> Optional[Dict[str, Any]]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        data = health_check(url, timeout=min(interval, 2.0))
        if data is not None:
            return data
        time.sleep(interval)
    return None


# ─── Public start / stop ─────────────────────────────────────────────────────


def start_server(
    binary: PathLike,
    port: int = DEFAULT_PORT,
    data_dir: Optional[PathLike] = None,
    *,
    host: str = DEFAULT_HOST,
    wait_timeout: float = 10.0,
) -> Dict[str, Any]:
    """Start local server under lock; write pid; wait for health.

    Returns dict: pid, port, url, data_dir, health.
    Raises AlreadyRunning, PortInUse, LockError, DaemonError, FileNotFoundError.
    """
    binary_path = Path(binary)
    if not binary_path.exists():
        raise FileNotFoundError(f"Binary not found: {binary_path}")

    port = int(port)
    url = f"http://{host}:{port}"
    health = health_check(url)
    if health is not None and (
        health.get("service") == "hipcortex"
        or health.get("status") == "ok"
        or "version" in health
    ):
        raise AlreadyRunning(f"HipCortex already running on {url}")

    if is_port_open(host, port):
        raise PortInUse(f"Port {port} already in use by another application")

    # Clean stale PID if process gone
    old = read_pid()
    if old is not None and not is_pid_alive(old):
        clear_pid()
    elif old is not None and is_pid_alive(old):
        raise AlreadyRunning(f"HipCortex daemon already running (PID {old})")

    if not acquire_lock(owner_pid=os.getpid()):
        holder = _read_lock_pid()
        raise LockError(
            f"Could not acquire {LOCK_FILE}"
            + (f" (held by PID {holder})" if holder is not None else "")
        )

    resolved_data = resolve_data_dir(data_dir)
    env = os.environ.copy()
    env["PORT"] = str(port)
    env["DATA_DIR"] = str(resolved_data)
    env.setdefault("RUST_LOG", "warn")

    try:
        proc = subprocess.Popen(
            [str(binary_path)],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except OSError as exc:
        release_lock()
        raise DaemonError(f"Failed to spawn server: {exc}") from exc

    write_pid(proc.pid)
    # Re-stamp lock with server pid so stop can reason about owner
    try:
        LOCK_FILE.write_text(f"{proc.pid}\n", encoding="utf-8")
    except OSError:
        pass

    health = wait_for_health(url, timeout=wait_timeout)
    if health is None:
        # Leave process running (may still be starting); caller can check status.
        return {
            "pid": proc.pid,
            "port": port,
            "url": url,
            "data_dir": str(resolved_data),
            "health": None,
            "ready": False,
        }

    return {
        "pid": proc.pid,
        "port": port,
        "url": url,
        "data_dir": str(resolved_data),
        "health": health,
        "ready": True,
    }


def stop_server(port: int = DEFAULT_PORT, *, host: str = DEFAULT_HOST) -> Dict[str, Any]:
    """Stop server via PID file first, then port fallback. Releases lock.

    Returns dict: stopped, pid, method (pid|port|none).
    """
    port = int(port)
    stopped = False
    method = "none"
    used_pid: Optional[int] = None

    pid = read_pid()
    if pid is not None:
        if is_pid_alive(pid):
            _terminate_pid(pid)
            for _ in range(10):
                time.sleep(0.3)
                if not is_pid_alive(pid):
                    break
            else:
                _force_kill_pid(pid)
                time.sleep(0.2)
            stopped = True
            method = "pid"
            used_pid = pid
            clear_pid()
            release_lock()
            return {"stopped": True, "pid": used_pid, "method": method}
        # Stale pid file
        clear_pid()

    # Port fallback
    for p in _pids_on_port(port):
        _terminate_pid(p)
        for _ in range(8):
            time.sleep(0.25)
            if not is_pid_alive(p):
                break
        else:
            _force_kill_pid(p)
        stopped = True
        method = "port"
        used_pid = p

    clear_pid()
    release_lock()

    # Also clear if nothing was listening but lock/pid leftovers exist
    if not stopped and not is_port_open(host, port):
        return {"stopped": False, "pid": None, "method": "none"}

    return {"stopped": stopped, "pid": used_pid, "method": method}


def restart_server(
    binary: PathLike,
    port: int = DEFAULT_PORT,
    data_dir: Optional[PathLike] = None,
    *,
    host: str = DEFAULT_HOST,
    wait_timeout: float = 10.0,
) -> Dict[str, Any]:
    """Stop then start. Ignores stop-not-found."""
    stop_server(port=port, host=host)
    # brief settle for port release
    time.sleep(0.4)
    return start_server(
        binary,
        port=port,
        data_dir=data_dir,
        host=host,
        wait_timeout=wait_timeout,
    )
