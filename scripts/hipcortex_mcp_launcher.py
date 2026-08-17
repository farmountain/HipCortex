#!/usr/bin/env python3
"""HipCortex MCP launcher — starts Rust server if needed, then execs MCP stdio server."""
import os, socket, subprocess, sys, time

HIPCORTEX_DIR = r"D:\all_projects\hipcortex"
SERVER_BIN    = os.path.join(HIPCORTEX_DIR, "target", "release", "webserver.exe")
MCP_SERVER    = os.path.join(HIPCORTEX_DIR, "sdk", "mcp", "server.py")
PORT          = int(os.getenv("HIPCORTEX_PORT", "3030"))
LOG           = os.path.join(os.environ.get("TEMP", "C:/Temp"), "hipcortex-server.log")

def _port_open(port: int) -> bool:
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=1):
            return True
    except OSError:
        return False

if not _port_open(PORT):
    subprocess.Popen(
        [SERVER_BIN],
        stdout=open(LOG, "w"),
        stderr=subprocess.STDOUT,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    for _ in range(20):
        time.sleep(0.5)
        if _port_open(PORT):
            break

# Replace this process with the MCP stdio server
os.execv(sys.executable, [sys.executable, MCP_SERVER])
