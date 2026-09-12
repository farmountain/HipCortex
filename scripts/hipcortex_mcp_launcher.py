#!/usr/bin/env python3
"""HipCortex MCP launcher — starts Rust server if needed, then serves MCP on stdio."""
import os, runpy, socket, subprocess, sys, time

# Repo root is this script's grandparent directory. Derived rather than hard-coded so the
# launcher is correct on any checkout path and on case-sensitive filesystems. The previous
# literal spelled the directory "hipcortex" in lowercase, which only resolved because
# Windows paths are case-insensitive.
HIPCORTEX_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
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

# The Rust backend is a convenience, not a prerequisite: the MCP stdio server still answers
# tool calls and reports backend errors per-call when the port is down. A missing or
# unlaunchable binary must therefore degrade to "backend absent", never kill this process —
# otherwise the host shows a dead MCP server instead of a degraded one.
if not _port_open(PORT):
    try:
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
    except OSError as exc:
        print(f"hipcortex launcher: backend not started ({exc}); "
              f"continuing with MCP stdio only", file=sys.stderr, flush=True)

if not os.path.exists(MCP_SERVER):
    # Fatal and unambiguous: there is no server to run, so say exactly which path is wrong
    # rather than letting the host report only "process exited with code N".
    print(f"hipcortex launcher: MCP server not found at {MCP_SERVER}",
          file=sys.stderr, flush=True)
    raise SystemExit(1)

# Flush explicitly so the diagnostics above are not lost when the server takes over.
sys.stdout.flush()
sys.stderr.flush()

# Serve in THIS process rather than handing off to a replacement.
#
# os.execv on Windows does not replace the image: it spawns a new process and terminates
# this one. An MCP host tracks the pid it spawned, so the hand-off is observed as "the
# server exited" even while the replacement is still answering on the inherited pipes.
# runpy keeps the host's pid as the one speaking the protocol, and needs no child process
# for stdio, whose lifetime is exactly this process's lifetime.
runpy.run_path(MCP_SERVER, run_name="__main__")
