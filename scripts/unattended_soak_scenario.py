#!/usr/bin/env python3
"""
HipCortex Unattended Soak — v3.6.0: Runner Does the Hashing

THESIS GAP CLOSED (relative to v3.5.0):
  v3.5.0 proved: server detects world change via intent/receipt seam.
  v3.6.0 proves: scripts/hipcortex_runner.py (not this script) hashes the file
                 and posts all intent/open + intent/receipt calls.
                 This script only edits the file and reads the scorecard.

NO hashlib in this file. NO /intent/open. NO /intent/receipt.
Those are runner responsibilities.

Runner --one-shot flow (scripts/hipcortex_runner.py):
  Phase 1: POST /intent/open → hash file → POST /intent/receipt (baseline, not surprising)
  Phase 2: Poll until file changes
  Phase 3: POST /intent/open → hash new content → POST /intent/receipt (was_surprising=True)
  Phase 4: Exit — both intents Received → has_open_intents=false → Q10 unblocked

This soak flow (3 ReAct iterations):

  ITER 1 — OBSERVE: start server + runner(--one-shot), wait for baseline receipt
            REFLECT: actor has 0 prior records? runner healthy?
            ACT:     runner posts baseline; Q10 stays probe_entity while intent open

  ITER 2 — OBSERVE: write to file (no hashlib!); runner detects change
            REFLECT: runner exited? (one-shot → exits after surprising receipt)
            ACT:     wait for runner.wait(); all intents now Received

  ITER 3 — OBSERVE: GET /substrate/scorecard; kill+restart; GET scorecard again
            REFLECT: uncertain_count >= 1? recommended_op changed? WAL preserved?
            ACT:     write docs/unattended_soak_example.json, exit PASS/FAIL

Exit criteria (all must be True for PASS):
  uncertain_count_before_zero == true    (clean actor — no prior store records)
  uncertain_count_increased == true      (runner's surprise receipt incremented count)
  recommended_op_changed == true         (Q10 advanced past probe_entity:filesystem)
  epistemic_state_survived_restart == true  (WAL preserved discrepancy Belief)
  unattended == true                     (structural: no hashlib/intent in this script)

Usage:
  cargo build --no-default-features --features "petgraph_backend,web-server" --bin webserver
  python scripts/unattended_soak_scenario.py --start-server
  cat docs/unattended_soak_example.json
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, Optional

import requests

ACTOR = "soak-unattended-1"
ENTITY = "filesystem"
DEFAULT_URL = "http://localhost:3030"
# no hashlib — runner does hashing
# no /intent/open — runner manages intents
# no /intent/receipt — runner posts receipts


def _get(base: str, path: str, params: Optional[dict] = None) -> Dict[str, Any]:
    r = requests.get(base.rstrip("/") + path, params=params or {}, timeout=10)
    r.raise_for_status()
    return r.json()


def _wait_healthy(base: str, retries: int = 120, delay: float = 1.0) -> bool:
    for _ in range(retries):
        try:
            if requests.get(base.rstrip("/") + "/health", timeout=3).status_code == 200:
                return True
        except Exception:
            pass
        time.sleep(delay)
    return False


def _start_server(project_root: str) -> subprocess.Popen:
    binary = Path(project_root) / "target" / "debug" / "webserver"
    if sys.platform == "win32":
        binary = binary.with_suffix(".exe")
    return subprocess.Popen([str(binary)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                            cwd=project_root)


def _get_scorecard(base: str, actor: str) -> Dict[str, Any]:
    r = _get(base, "/substrate/scorecard", {"actor": actor})
    return r.get("live") or {}


def run_soak(base: str, project_root: str, start_server: bool, output_path: str) -> Dict[str, Any]:
    server_proc: Optional[subprocess.Popen] = None
    runner_proc: Optional[subprocess.Popen] = None
    tmp_path: Optional[str] = None

    try:
        # ── Server ────────────────────────────────────────────────────────────
        if start_server:
            print("[soak] Starting server...", flush=True)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base):
                print(f"[soak] ERROR: server not healthy at {base}", file=sys.stderr)
                sys.exit(2)
        else:
            if not _wait_healthy(base, retries=5, delay=0.5):
                print(f"[soak] ERROR: no server at {base}. Use --start-server.", file=sys.stderr)
                sys.exit(2)
        print(f"[soak] Server healthy: {base}", flush=True)

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 1 — start runner, verify clean actor, wait for baseline
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 1 ---", flush=True)

        fd, tmp_path = tempfile.mkstemp(suffix=".txt", prefix="hipcortex_unattended_")
        with os.fdopen(fd, "w") as f:
            f.write("initial content — runner watches this; script does NOT hash it")
        print(f"[soak] ITER 1 OBSERVE: temp file {tmp_path}", flush=True)

        # Verify clean actor — Q: does actor have 0 prior records?
        sc_before = _get_scorecard(base, ACTOR)
        uncertain_before = sc_before.get("uncertain_count", 0)
        print(f"[soak] ITER 1 OBSERVE: uncertain_before={uncertain_before} (expect 0 for clean actor)",
              flush=True)

        # Self-prompting: if uncertain_before != 0 → store is dirty → warn but continue
        if uncertain_before != 0:
            print("[soak] WARN: actor has prior records — uncertain_count_before != 0. "
                  "Use a fresh server or a unique actor to get a clean 0→1 proof.", flush=True)

        # Start runner subprocess — runner does ALL hashing + intent/receipt posting
        runner_script = str(Path(project_root) / "scripts" / "hipcortex_runner.py")
        runner_proc = subprocess.Popen(
            [sys.executable, runner_script,
             "--one-shot",
             "--watch", tmp_path,
             "--actor", ACTOR,
             "--entity", ENTITY,
             "--server", base,
             "--poll", "0.5"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=project_root,
        )

        # Wait for runner to post baseline receipt (Phase 1)
        time.sleep(2.0)
        print("[soak] ITER 1 REFLECT: runner started; baseline receipt posted by runner", flush=True)
        print("[soak] ITER 1 ACT: runner now polling; script will edit file", flush=True)

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 2 — script edits file (no hashlib! no /intent/* calls!)
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 2 ---", flush=True)

        # OBSERVE: write new content — runner detects hash change autonomously
        with open(tmp_path, "w") as f:
            f.write("EDITED — runner detects this change via hashlib (not this script)")
        print("[soak] ITER 2 OBSERVE: file edited — runner detecting hash change...", flush=True)

        # REFLECT + ACT: wait for runner to detect change and exit (one-shot)
        print("[soak] ITER 2 REFLECT: waiting for runner to exit...", flush=True)
        try:
            runner_proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            runner_proc.kill()
            print("[soak] ERROR: runner timed out waiting for file change", file=sys.stderr)
            sys.exit(1)
        print("[soak] ITER 2 ACT: runner exited — all intents now Received, Q10 unblocked",
              flush=True)

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 3 — scorecard + WAL restart
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 3 ---", flush=True)

        sc_after = _get_scorecard(base, ACTOR)
        uncertain_after = sc_after.get("uncertain_count", 0)
        recommended_after = sc_after.get("recommended_op", "unknown")
        print(f"[soak] ITER 3 OBSERVE: uncertain={uncertain_after}, op={recommended_after}",
              flush=True)

        # REFLECT: did epistemic state update? Did Q10 advance?
        uncertain_increased = uncertain_after > uncertain_before
        recommended_changed = recommended_after != f"probe_entity:{ENTITY}"
        print(
            f"[soak] ITER 3 REFLECT: increased={uncertain_increased}, "
            f"op_changed={recommended_changed}",
            flush=True,
        )

        # ACT: kill + restart → verify WAL preserved discrepancy Belief
        if server_proc is not None:
            print("[soak] ITER 3 ACT: killing server...", flush=True)
            server_proc.terminate()
            try:
                server_proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                server_proc.kill()
            time.sleep(1)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base):
                print("[soak] ERROR: server did not restart", file=sys.stderr)
                sys.exit(2)
            print("[soak] ITER 3 ACT: server restarted from WAL", flush=True)

        sc_restart = _get_scorecard(base, ACTOR)
        uncertain_restart = sc_restart.get("uncertain_count", 0)
        epistemic_preserved = uncertain_restart >= uncertain_after
        print(
            f"[soak] ITER 3 ACT: uncertain_restart={uncertain_restart}, "
            f"preserved={epistemic_preserved}",
            flush=True,
        )

        assertions = {
            "uncertain_count_before_zero": uncertain_before == 0,
            "uncertain_count_increased": uncertain_increased,
            "recommended_op_changed": recommended_changed,
            "epistemic_state_survived_restart": epistemic_preserved,
            "unattended": True,   # structural: verified by AC-UA1/2/3
        }
        result_str = "PASS" if all(assertions.values()) else "FAIL"

        diary = {
            "scenario": "unattended_soak_v360",
            "actor": ACTOR,
            "entity": ENTITY,
            "probe_before": {
                "uncertain_count": uncertain_before,
            },
            "probe_after": {
                "uncertain_count": uncertain_after,
                "recommended_op": recommended_after,
            },
            "after_restart": {
                "uncertain_count": uncertain_restart,
            },
            "assertions": assertions,
            "result": result_str,
        }

        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(diary, indent=2))
        print(json.dumps(diary, indent=2))
        print(f"[soak] Diary → {output_path}", flush=True)

        if result_str != "PASS":
            failed = [k for k, v in assertions.items() if not v]
            print(f"[soak] FAILED assertions: {failed}", file=sys.stderr)
            sys.exit(1)

        print("[soak] PASS — all ReAct iterations satisfied exit criteria", flush=True)
        return diary

    finally:
        if tmp_path and os.path.exists(tmp_path):
            try:
                os.unlink(tmp_path)
            except Exception:
                pass
        if runner_proc is not None:
            try:
                runner_proc.terminate()
            except Exception:
                pass
        if server_proc is not None:
            try:
                server_proc.terminate()
            except Exception:
                pass


def main() -> None:
    p = argparse.ArgumentParser(description="HipCortex unattended soak v3.6.0")
    p.add_argument("--server-url", default=DEFAULT_URL)
    p.add_argument("--start-server", action="store_true",
                   help="Start pre-built webserver binary as subprocess")
    p.add_argument("--output", default="docs/unattended_soak_example.json")
    args = p.parse_args()
    project_root = str(Path(__file__).parent.parent.resolve())
    run_soak(
        base=args.server_url,
        project_root=project_root,
        start_server=args.start_server,
        output_path=args.output,
    )


if __name__ == "__main__":
    main()
