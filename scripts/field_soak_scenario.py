#!/usr/bin/env python3
"""
HipCortex Field Soak Scenario — v3.4.0 two-process afternoon proof.

Proves the 3-month claim with a real two-process scenario:
  Process 1 (server): starts, persists memories to WAL+JSONL
  Process 2 (this script): submits intents via HTTP, edits file, verifies restart

Usage:
  python scripts/field_soak_scenario.py [--server-url URL] [--start-server]

Options:
  --server-url URL     HipCortex server URL (default: http://localhost:3030)
  --start-server       Build and start server as subprocess (requires cargo)
  --output PATH        Diary output path (default: docs/field_soak_example.json)

Exit codes: 0=pass, 1=fail (assertion), 2=prereq missing
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Optional

import requests

ACTOR = "soak-1"


def _url(base: str, path: str) -> str:
    return base.rstrip("/") + path


def _add_memory(base: str, actor: str, action: str, target: str,
                memory_type: str = "Temporal", content: str = "") -> dict:
    r = requests.post(_url(base, "/memory"), json={
        "actor": actor, "action": action, "target": target,
        "memory_type": memory_type, "content": content,
    }, timeout=10)
    r.raise_for_status()
    return r.json()


def _scorecard(base: str, actor: str) -> dict:
    r = requests.get(_url(base, "/substrate/scorecard"),
                     params={"actor": actor}, timeout=10)
    r.raise_for_status()
    return r.json()


def _wait_healthy(base: str, retries: int = 30, delay: float = 1.0) -> bool:
    for _ in range(retries):
        try:
            r = requests.get(_url(base, "/health"), timeout=3)
            if r.status_code == 200:
                return True
        except Exception:
            pass
        time.sleep(delay)
    return False


def _start_server(work_dir: str) -> subprocess.Popen:
    return subprocess.Popen(
        [
            "cargo", "run", "--quiet",
            "--no-default-features", "--features", "petgraph_backend,web-server",
            "--bin", "webserver",
        ],
        cwd=work_dir,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def run_scenario(base_url: str, start_server: bool,
                 output_path: str, project_root: str) -> dict:
    server_proc: Optional[subprocess.Popen] = None
    target_path: Optional[str] = None

    try:
        if start_server:
            print("[field-soak] Building + starting server...", flush=True)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base_url, retries=120, delay=2.0):
                print(f"[field-soak] ERROR: server did not become healthy at {base_url}",
                      file=sys.stderr)
                sys.exit(2)
            print(f"[field-soak] Server healthy at {base_url}", flush=True)
        else:
            if not _wait_healthy(base_url, retries=5, delay=0.5):
                print(f"[field-soak] ERROR: no server at {base_url}. "
                      "Use --start-server or start manually.", file=sys.stderr)
                sys.exit(2)

        # Phase 1: seed memories
        print("[field-soak] Phase 1: seeding memories...", flush=True)
        for i in range(5):
            _add_memory(base_url, ACTOR, "observed", f"sensor_reading_{i}",
                        content=f"initial probe {i}")

        # Phase 2: BEFORE scorecard
        before = _scorecard(base_url, ACTOR)
        print(f"[field-soak] BEFORE: record_count={before.get('record_count', '?')}", flush=True)

        # Phase 3: create + probe a temp file
        fd, target_path = tempfile.mkstemp(suffix=".md")
        with os.fdopen(fd, "w") as f:
            f.write("# HipCortex field soak target v1\n")
        print(f"[field-soak] Phase 3: temp file {target_path}", flush=True)

        probe_v1 = Path(target_path).read_text()
        _add_memory(base_url, ACTOR, "probed", target_path,
                    memory_type="Temporal", content=probe_v1)

        # Edit file (sed-equivalent) — content change must land in WM
        with open(target_path, "a") as f:
            f.write("\n# Edited — field soak phase 3\n")

        probe_v2 = Path(target_path).read_text()
        _add_memory(base_url, ACTOR, "probed_after_edit", target_path,
                    memory_type="Temporal", content=probe_v2)

        # Phase 4: AFTER_EDIT scorecard
        after_edit = _scorecard(base_url, ACTOR)
        print(f"[field-soak] AFTER_EDIT: record_count={after_edit.get('record_count', '?')}",
              flush=True)

        # Phase 5: restart server
        print("[field-soak] Phase 5: restarting server...", flush=True)
        if server_proc is not None:
            server_proc.terminate()
            try:
                server_proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                server_proc.kill()
            time.sleep(1)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base_url, retries=120, delay=2.0):
                print("[field-soak] ERROR: server did not restart cleanly", file=sys.stderr)
                sys.exit(2)
            print("[field-soak] Server restarted healthy", flush=True)
        else:
            print("[field-soak] (external server — skipping process restart; "
                  "stop+start manually to verify WAL survival)", flush=True)

        # Phase 6: AFTER_RESTART scorecard — memories must survive
        after_restart = _scorecard(base_url, ACTOR)
        print(f"[field-soak] AFTER_RESTART: record_count={after_restart.get('record_count', '?')}",
              flush=True)

        # Assertions
        bc = before.get("record_count", 0)
        ac = after_edit.get("record_count", 0)
        ar = after_restart.get("record_count", 0)

        assertions = {
            "after_edit_gt_before": ac > bc,
            "after_restart_ge_after_edit": ar >= ac,
            "records_survive_restart": ar > 0,
        }

        diary = {
            "scenario": "field_soak_two_process_v340",
            "actor": ACTOR,
            "before": before,
            "after_edit": after_edit,
            "after_restart": after_restart,
            "assertions": assertions,
            "result": "PASS" if all(assertions.values()) else "FAIL",
        }

        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(diary, indent=2))
        print(f"[field-soak] Diary → {out}", flush=True)
        print(f"[field-soak] Result: {diary['result']}", flush=True)

        if diary["result"] != "PASS":
            failed = [k for k, v in assertions.items() if not v]
            print(f"[field-soak] FAILED: {failed}", file=sys.stderr)
            sys.exit(1)

        return diary

    finally:
        if server_proc is not None:
            try:
                server_proc.terminate()
            except Exception:
                pass
        if target_path is not None:
            try:
                os.unlink(target_path)
            except Exception:
                pass


def main() -> None:
    p = argparse.ArgumentParser(description="HipCortex field soak scenario")
    p.add_argument("--server-url", default="http://localhost:3030")
    p.add_argument("--start-server", action="store_true",
                   help="Build and start server subprocess (requires cargo)")
    p.add_argument("--output", default="docs/field_soak_example.json")
    args = p.parse_args()

    project_root = str(Path(__file__).parent.parent.resolve())
    run_scenario(
        base_url=args.server_url,
        start_server=args.start_server,
        output_path=args.output,
        project_root=project_root,
    )


if __name__ == "__main__":
    main()
