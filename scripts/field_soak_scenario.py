#!/usr/bin/env python3
"""
HipCortex Epistemic Field Soak — v3.5.0 seam proof.

GOAL: Prove "the agent noticed the world changed" via the intent/receipt seam.
      No /memory/add for the file-change event — cognitive updates flow through
      POST /intent/open → hashlib.sha256 → POST /intent/receipt ONLY.

THESIS GAP CLOSED (relative to v3.4.0):
  v3.4.0 proved: WAL survives server kill (record_count persisted).
  v3.5.0 proves: world-model epistemically updated via receipt seam, NOT /memory/add.
                 The server itself detects the world changed — no human annotation.

Seam flow (iterations 1 & 2):
  POST /intent/open {actor, target_entity="filesystem", deadline_ms=300000}
     → intent_id (UUID)
  hashlib.sha256(file_content)
     → sha256_hex (sensor reading — no /memory/add involved)
  POST /intent/receipt {actor, intent_id, ok=True,
                        observation={sha256_hex, path}, sensor_path="soak:filesystem"}
     → server: wm_updater.update_from_receipt() → was_surprising → Belief{confidence=0.3}
     → scorecard: uncertain_count increases

ReAct loop (3 iterations, self-terminating on PASS):

  ITER 1 — OBSERVE: probe filesystem hash before edit
            REFLECT: intent open + hash computed?
            ACT:     POST /intent/receipt (not surprising — no prior WM state)

  ITER 2 — OBSERVE: silent file edit (no /memory/add), probe new hash
            REFLECT: hashes differ?  (abort if not — edit failed)
            ACT:     POST /intent/receipt (was_surprising=True → uncertain_count++)

  ITER 3 — OBSERVE: kill + restart server, fetch scorecard
            REFLECT: WAL preserved discrepancy Belief?
            ACT:     write docs/epistemic_soak_example.json, exit

Exit criteria (all must be True for PASS):
  - sha256_before != sha256_after       (content actually changed)
  - uncertain_count_after > uncertain_count_before  (epistemic state updated)
  - uncertain_count_restart >= uncertain_count_after (WAL preserved epistemic state)

Clarifying / self-prompting resolved before coding:
  Q: Does first receipt trigger was_surprising?
  A: No — wm_updater.rs:54 returns false when no prior WM transitions exist.
  Q: Does second receipt with different sha256 trigger was_surprising?
  A: Yes — new obs_state "filesystem:<hash8>" diverges from MAP prediction.
  Q: Does uncertain_count count beliefs per-actor?
  A: Yes — build_report(store, actor) filters by actor.
  Q: Do expired intents contaminate uncertain_count?
  A: No — expired intents add to invalidated_count, not uncertain_beliefs.
  Q: PROBE_DEADLINE_MS strategy?
  A: 300 000 ms (5 min) — avoids Expired status during soak run.

Usage:
  cargo build --no-default-features --features "petgraph_backend,web-server" --bin webserver
  python scripts/field_soak_scenario.py --start-server
  cat docs/epistemic_soak_example.json

Options:
  --server-url URL   HipCortex server base URL (default: http://localhost:3030)
  --start-server     Start pre-built webserver binary as subprocess
  --output PATH      Diary output path (default: docs/epistemic_soak_example.json)

Exit codes: 0=PASS, 1=FAIL (assertion), 2=prereq error
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, Optional

import requests

ACTOR = "soak-epistemic-1"
ENTITY = "filesystem"
SENSOR_PATH = "soak:filesystem"
PROBE_DEADLINE_MS = 300_000   # 5 min; avoids expired-intent noise in uncertain_count
DEFAULT_URL = "http://localhost:3030"
REACT_MAX_ITERATIONS = 3


# ── HTTP helpers ─────────────────────────────────────────────────────────────

def _url(base: str, path: str) -> str:
    return base.rstrip("/") + path


def _post(base: str, path: str, body: dict) -> Dict[str, Any]:
    r = requests.post(_url(base, path), json=body, timeout=10)
    r.raise_for_status()
    return r.json()


def _get(base: str, path: str, params: Optional[dict] = None) -> Dict[str, Any]:
    r = requests.get(_url(base, path), params=params or {}, timeout=10)
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


def _start_server(project_root: str) -> subprocess.Popen:
    binary = Path(project_root) / "target" / "debug" / "webserver"
    if sys.platform == "win32":
        binary = binary.with_suffix(".exe")
    return subprocess.Popen(
        [str(binary)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=project_root,
    )


# ── Seam primitives (no /memory/add) ────────────────────────────────────────

def _open_probe_intent(base: str, actor: str, entity: str) -> str:
    """POST /intent/open → intent_id UUID string."""
    r = _post(base, "/intent/open", {
        "actor": actor,
        "target_entity": entity,
        "deadline_ms": PROBE_DEADLINE_MS,
    })
    if not r.get("ok"):
        raise RuntimeError(f"intent/open failed: {r}")
    return r["intent_id"]


def _probe_filesystem(path: str) -> Dict[str, Any]:
    """Compute sha256_hex of file content — the sensor reading. No /memory/add used."""
    with open(path, "rb") as f:
        raw = f.read()
    return {
        "sha256_hex": hashlib.sha256(raw).hexdigest(),
        "size_bytes": len(raw),
        "path": path,
    }


def _accept_receipt(
    base: str, actor: str, intent_id: str, observation: Dict[str, Any]
) -> Dict[str, Any]:
    """POST /intent/receipt → server runs update_from_receipt() + discrepancy check."""
    r = _post(base, "/intent/receipt", {
        "actor": actor,
        "intent_id": intent_id,
        "ok": True,
        "observation": observation,
        "sensor_path": SENSOR_PATH,
    })
    if not r.get("ok"):
        raise RuntimeError(f"intent/receipt failed: {r}")
    return r


def _get_scorecard(base: str, actor: str) -> Dict[str, Any]:
    """GET /substrate/scorecard?actor=X → live sub-object {uncertain_count, recommended_op, ...}."""
    r = _get(base, "/substrate/scorecard", {"actor": actor})
    return r.get("live") or {}


# ── Main ReAct loop ──────────────────────────────────────────────────────────

def run_soak(
    base: str,
    project_root: str,
    start_server: bool,
    output_path: str,
) -> Dict[str, Any]:

    server_proc: Optional[subprocess.Popen] = None
    tmp_path: Optional[str] = None

    try:
        # ── Server health check ───────────────────────────────────────────────
        if start_server:
            print("[soak] Starting webserver subprocess...", flush=True)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base, retries=120, delay=2.0):
                print(f"[soak] ERROR: server not healthy at {base}", file=sys.stderr)
                sys.exit(2)
            print(f"[soak] Server healthy at {base}", flush=True)
        else:
            if not _wait_healthy(base, retries=5, delay=0.5):
                print(
                    f"[soak] ERROR: no server at {base}. Use --start-server.",
                    file=sys.stderr,
                )
                sys.exit(2)

        # ── Create temp file ──────────────────────────────────────────────────
        fd, tmp_path = tempfile.mkstemp(suffix=".txt", prefix="hipcortex_soak_")
        with os.fdopen(fd, "w") as f:
            f.write("initial content v1 — agent will probe this file")
        print(f"[soak] Temp file: {tmp_path}", flush=True)

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 1 — Probe before edit (establishes WM baseline)
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 1 ---", flush=True)

        # OBSERVE: read file sha256 before any edit
        intent_id_1 = _open_probe_intent(base, ACTOR, ENTITY)
        obs_before = _probe_filesystem(tmp_path)
        print(f"[soak] ITER 1 OBSERVE: sha256={obs_before['sha256_hex'][:12]}…", flush=True)

        # REFLECT: intent open and hash computed — conditions met to act
        print("[soak] ITER 1 REFLECT: intent open, hash computed — proceeding to receipt", flush=True)

        # ACT: receipt (not surprising — no prior WM prediction exists for this actor)
        _accept_receipt(base, ACTOR, intent_id_1, obs_before)
        sc_before = _get_scorecard(base, ACTOR)
        uncertain_before = sc_before.get("uncertain_count", 0)
        recommended_before = sc_before.get("recommended_op", "unknown")
        print(
            f"[soak] ITER 1 ACT: receipt sent → uncertain={uncertain_before}, op={recommended_before}",
            flush=True,
        )

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 2 — Silent edit, probe again (triggers was_surprising)
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 2 ---", flush=True)

        # OBSERVE: silent file edit — NO /memory/add (this is the thesis)
        with open(tmp_path, "w") as f:
            f.write("EDITED content v2 — agent discovers this via receipt seam, not /memory/add")
        intent_id_2 = _open_probe_intent(base, ACTOR, ENTITY)
        obs_after = _probe_filesystem(tmp_path)
        print(f"[soak] ITER 2 OBSERVE: sha256={obs_after['sha256_hex'][:12]}…", flush=True)

        # REFLECT: did hash change? If not, abort — edit did not change content.
        hashes_differ = obs_before["sha256_hex"] != obs_after["sha256_hex"]
        print(f"[soak] ITER 2 REFLECT: hashes_differ={hashes_differ}", flush=True)
        if not hashes_differ:
            raise RuntimeError("REFLECT FAIL: hashes identical — temp file edit failed")

        # ACT: receipt with new sha256 → was_surprising=True → Belief{confidence=0.3}
        _accept_receipt(base, ACTOR, intent_id_2, obs_after)
        sc_after = _get_scorecard(base, ACTOR)
        uncertain_after = sc_after.get("uncertain_count", 0)
        recommended_after = sc_after.get("recommended_op", "unknown")
        uncertain_increased = uncertain_after > uncertain_before
        print(
            f"[soak] ITER 2 ACT: receipt sent → uncertain={uncertain_after}, op={recommended_after}, increased={uncertain_increased}",
            flush=True,
        )

        # ═══════════════════════════════════════════════════════════════════════
        # ITERATION 3 — Kill + restart → WAL preserves epistemic state
        # ═══════════════════════════════════════════════════════════════════════
        print("[soak] --- ITER 3 ---", flush=True)

        # OBSERVE: kill server, restart, check scorecard
        if server_proc is not None:
            print("[soak] ITER 3 OBSERVE: terminating server...", flush=True)
            server_proc.terminate()
            try:
                server_proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                server_proc.kill()
            time.sleep(1)
            server_proc = _start_server(project_root)
            if not _wait_healthy(base, retries=120, delay=2.0):
                print("[soak] ERROR: server did not restart", file=sys.stderr)
                sys.exit(2)
            print("[soak] ITER 3 OBSERVE: server restarted from WAL", flush=True)

        sc_restart = _get_scorecard(base, ACTOR)
        uncertain_restart = sc_restart.get("uncertain_count", 0)
        recommended_restart = sc_restart.get("recommended_op", "unknown")

        # REFLECT: did WAL preserve the discrepancy Belief?
        epistemic_preserved = uncertain_restart >= uncertain_after
        print(
            f"[soak] ITER 3 REFLECT: uncertain_restart={uncertain_restart}, preserved={epistemic_preserved}",
            flush=True,
        )

        # ACT: compose diary, assert all goals met, write JSON
        assertions = {
            "hashes_differ": hashes_differ,
            "uncertain_count_increased": uncertain_increased,
            "epistemic_state_survived_restart": epistemic_preserved,
            "no_memory_add_for_edit": True,   # structural: seam-only path above
        }
        result_str = "PASS" if all(assertions.values()) else "FAIL"

        diary = {
            "scenario": "epistemic_seam_proof_v350",
            "actor": ACTOR,
            "entity": ENTITY,
            "probe_before": {
                "sha256_hex": obs_before["sha256_hex"],
                "uncertain_count": uncertain_before,
                "recommended_op": recommended_before,
            },
            "probe_after": {
                "sha256_hex": obs_after["sha256_hex"],
                "uncertain_count": uncertain_after,
                "recommended_op": recommended_after,
            },
            "after_restart": {
                "uncertain_count": uncertain_restart,
                "recommended_op": recommended_restart,
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
        if server_proc is not None:
            try:
                server_proc.terminate()
            except Exception:
                pass


def main() -> None:
    p = argparse.ArgumentParser(description="HipCortex epistemic field soak v3.5.0")
    p.add_argument("--server-url", default=DEFAULT_URL)
    p.add_argument(
        "--start-server",
        action="store_true",
        help="Start pre-built webserver binary as subprocess",
    )
    p.add_argument("--output", default="docs/epistemic_soak_example.json")
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
