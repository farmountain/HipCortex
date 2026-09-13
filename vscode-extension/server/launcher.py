#!/usr/bin/env python3
"""HipCortex MCP launcher — VSIX edition.

All paths derived from this file's location inside the installed extension.
Starts the Rust HTTP backend if not already running, then execs the MCP stdio server.
"""
import os
import platform as _platform
import runpy
import socket
import subprocess
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
MCP_SERVER  = os.path.join(SCRIPT_DIR, "mcp_server.py")

# ── Platform binary selection ─────────────────────────────────────────────────
_sys  = _platform.system().lower()
_arch = _platform.machine().lower()

if _sys == "windows":
    _plat = "win32"
    _bin  = "hipcortex-windows-amd64.exe" if ("x86_64" in _arch or "amd64" in _arch) else "hipcortex-windows-arm64.exe"
elif _sys == "darwin":
    _plat = "darwin"
    _bin  = "hipcortex-macos-arm64" if "arm" in _arch else "hipcortex-macos-amd64"
else:
    _plat = "linux"
    _bin  = "hipcortex-linux-arm64" if "arm" in _arch else "hipcortex-linux-amd64"

SERVER_BIN = os.path.join(SCRIPT_DIR, _plat, _bin)
PORT       = int(os.getenv("HIPCORTEX_PORT", "3030"))
_tmp       = os.environ.get("TEMP") or os.environ.get("TMPDIR") or "/tmp"
LOG        = os.path.join(_tmp, "hipcortex-server.log")


def _port_open(p: int) -> bool:
    try:
        with socket.create_connection(("127.0.0.1", p), timeout=0.5):
            return True
    except OSError:
        return False


if not _port_open(PORT):
    if not os.path.isfile(SERVER_BIN):
        print(f"hipcortex launcher: binary not found: {SERVER_BIN}", file=sys.stderr, flush=True)
    else:
        try:
            flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
            subprocess.Popen(
                [SERVER_BIN],
                stdout=open(LOG, "w"),
                stderr=subprocess.STDOUT,
                creationflags=flags,
            )
            for _ in range(20):
                time.sleep(0.5)
                if _port_open(PORT):
                    break
        except OSError as exc:
            print(f"hipcortex launcher: could not start backend: {exc}", file=sys.stderr, flush=True)

if not os.path.isfile(MCP_SERVER):
    print(f"hipcortex launcher: MCP server not found: {MCP_SERVER}", file=sys.stderr, flush=True)
    raise SystemExit(1)

sys.stdout.flush()
sys.stderr.flush()
runpy.run_path(MCP_SERVER, run_name="__main__")
