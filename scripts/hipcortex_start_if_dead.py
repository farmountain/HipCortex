#!/usr/bin/env python3
"""SessionStart hook: ensure HipCortex REST backend is alive on port 3030."""
import os
import socket
import subprocess
import sys
import time

HIPCORTEX_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SERVER_BIN = os.path.join(HIPCORTEX_DIR, "target", "release", "webserver.exe")
PORT = int(os.getenv("HIPCORTEX_PORT", "3030"))
LOG_PATH = os.path.join(os.environ.get("TEMP", "C:/Temp"), "hipcortex-server.log")


def _port_open(port: int) -> bool:
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=1.0):
            return True
    except OSError:
        return False


def main() -> None:
    if _port_open(PORT):
        print(f"[hipcortex] server already running on :{PORT}", flush=True)
        return

    if not os.path.isfile(SERVER_BIN):
        print(f"[hipcortex] binary not found at {SERVER_BIN} — skipping start", flush=True)
        return

    try:
        with open(LOG_PATH, "w") as log:
            subprocess.Popen(
                [SERVER_BIN],
                stdout=log,
                stderr=subprocess.STDOUT,
                cwd=HIPCORTEX_DIR,
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
    except OSError as exc:
        print(f"[hipcortex] failed to spawn binary: {exc}", flush=True)
        return

    for i in range(20):
        time.sleep(0.5)
        if _port_open(PORT):
            print(f"[hipcortex] server started (took {(i + 1) * 0.5:.1f}s)", flush=True)
            return

    print("[hipcortex] server did not come up in 10s — continuing without backend", flush=True)


if __name__ == "__main__":
    main()
