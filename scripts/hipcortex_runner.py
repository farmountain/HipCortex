#!/usr/bin/env python3
"""
HipCortex Autonomous File Runner — v3.7.0

Watches a file and autonomously posts /intent/open + /intent/receipt.
The runner IS the sensor: no external script involvement in hashing.

Modes:
  --one-shot   Baseline receipt → poll until change → surprising receipt → exit.
               Leaves all intents Received → Q10 advances past probe_entity:X.
               Used by unattended_soak_scenario.py.
  --guided     Daemon reads scorecard recommended_op, probes accordingly, calls
               POST /goal/:id/react, exits when goal status = Succeeded.
               Used by longrun_soak_scenario.py.
  (default)    Continuous: keeps cycling open→receipt→stable→new-cycle indefinitely.

Usage:
  # One-shot (for unattended soak):
  python scripts/hipcortex_runner.py --one-shot --watch /tmp/soak.txt --actor soak-runner-1

  # Guided (for long-run goal completion):
  python scripts/hipcortex_runner.py --guided --goal-id <uuid> --watch /tmp/watched.txt --actor my-runner

  # Continuous daemon:
  python scripts/hipcortex_runner.py --watch /path/to/file --actor my-runner
"""

import argparse
import hashlib
import signal
import sys
import time

import requests

_running = True


def _sha256(path: str) -> str:
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def _post(base: str, path: str, body: dict) -> dict:
    r = requests.post(base.rstrip("/") + path, json=body, timeout=10)
    r.raise_for_status()
    return r.json()


def _get(base: str, path: str, params: dict | None = None) -> dict:
    r = requests.get(base.rstrip("/") + path, params=params, timeout=10)
    r.raise_for_status()
    return r.json()


def _open_intent(base: str, actor: str, entity: str) -> str:
    r = _post(base, "/intent/open", {
        "actor": actor,
        "target_entity": entity,
        "deadline_ms": 600_000,
    })
    if not r.get("ok"):
        raise RuntimeError(f"/intent/open failed: {r}")
    return r["intent_id"]


def _send_receipt(base: str, actor: str, intent_id: str, sha256_hex: str, sensor_path: str,
                  content_excerpt: str = "") -> None:
    _post(base, "/intent/receipt", {
        "actor": actor,
        "intent_id": intent_id,
        "ok": True,
        "observation": {"sha256_hex": sha256_hex, "content_excerpt": content_excerpt},
        "sensor_path": sensor_path,
    })


def _get_scorecard(base: str, actor: str) -> dict:
    return _get(base, "/substrate/scorecard", {"actor": actor})


def _read_excerpt(path: str, max_bytes: int = 256) -> str:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return f.read(max_bytes)
    except OSError:
        return ""


def _poll_and_receipt(base: str, actor: str, entity: str, watch: str,
                      allow_open: bool = True) -> None:
    """Receipt daemon-opened intents for entity.

    allow_open=False (guided/production mode): pure sensing — never opens intents.
      If no pending intents found, logs and returns. Daemon must open first.
    allow_open=True (default): opens as fallback when no daemon intents pending.
    """
    sensor_path = f"runner:{entity}"
    try:
        r = _get(base, "/intent/open", {"actor": actor})
        pending = [
            i for i in r.get("intents", [])
            if i.get("target_entity") == entity
        ]
    except Exception as e:
        print(f"[runner] poll error: {e}", flush=True)
        pending = []

    if pending:
        h = _sha256(watch)
        excerpt = _read_excerpt(watch)
        for intent in pending:
            try:
                _send_receipt(base, actor, intent["id"], h, sensor_path, content_excerpt=excerpt)
                print(f"[runner] receipt daemon-intent={str(intent['id'])[:8]}... entity={entity}", flush=True)
            except Exception as e:
                print(f"[runner] receipt error: {e}", flush=True)
    elif allow_open:
        # Fallback: daemon has not opened an intent yet — open one ourselves.
        try:
            intent_id = _open_intent(base, actor, entity)
            h = _sha256(watch)
            excerpt = _read_excerpt(watch)
            _send_receipt(base, actor, intent_id, h, sensor_path, content_excerpt=excerpt)
            print(f"[runner] fallback open+receipt entity={entity} hash={h[:12]}...", flush=True)
        except Exception as e:
            print(f"[runner] fallback error: {e}", flush=True)
    else:
        print(f"[runner] no daemon intents for entity={entity} — waiting (single-role mode)", flush=True)


def _react_step(base: str, goal_id: str) -> dict:
    return _post(base, f"/goal/{goal_id}/react", {})


def run_one_shot(base: str, actor: str, entity: str, watch: str, poll: float) -> None:
    """Baseline receipt → poll until change → surprising receipt → exit.

    Phase 1: open intent_A, hash file, post receipt (not surprising — no prior WM).
             intent_A → Received. Q10 unblocked briefly.
    Phase 2: poll until file content changes.
    Phase 3: open intent_B, hash new content, post receipt (was_surprising=True).
             uncertain_count++. intent_B → Received. Q10 unblocked.
    Phase 4: exit — all intents Received, recommended_op != probe_entity.
    """
    sensor_path = f"runner:{entity}"

    # Phase 1 — baseline (not surprising: no prior WM transitions for this actor)
    intent_a = _open_intent(base, actor, entity)
    hash_before = _sha256(watch)
    _send_receipt(base, actor, intent_a, hash_before, sensor_path)
    print(f"[runner] INIT hash={hash_before[:12]}... intent={intent_a[:8]}... baseline sent", flush=True)

    # Phase 2 — poll for change
    print("[runner] Polling for content change...", flush=True)
    while True:
        time.sleep(poll)
        try:
            h = _sha256(watch)
        except FileNotFoundError:
            continue
        if h != hash_before:
            break

    # Phase 3 — surprising receipt (was_surprising=True on server → Belief{confidence=0.3})
    intent_b = _open_intent(base, actor, entity)
    hash_after = _sha256(watch)
    _send_receipt(base, actor, intent_b, hash_after, sensor_path)
    print(f"[runner] CHANGE hash={hash_after[:12]}... intent={intent_b[:8]}... surprise sent", flush=True)
    print("[runner] ONE-SHOT done — both intents Received, Q10 unblocked", flush=True)


def run_guided(base: str, actor: str, entity: str, watch: str, poll: float, goal_id: str) -> None:
    """Guided daemon: reads scorecard recommended_op each cycle.

    - probe_entity:X  → open intent + receipt for that entity
    - react_loop      → POST /goal/:id/react; exit when status=Succeeded
    - other           → wait next poll cycle

    Exits when goal Succeeded or SIGTERM received.
    """
    global _running

    def _stop(*_):
        global _running
        _running = False

    signal.signal(signal.SIGTERM, _stop)

    print(f"[runner] GUIDED mode goal={goal_id} actor={actor} entity={entity}", flush=True)

    while _running:
        time.sleep(poll)
        try:
            card = _get_scorecard(base, actor)
        except Exception as e:
            print(f"[runner] scorecard error: {e}", flush=True)
            continue

        rec = card.get("next_recommendation", {})
        op = rec.get("recommended_op", "")
        print(f"[runner] scorecard op={op}", flush=True)

        if op.startswith("probe_entity:"):
            probe_entity = op.split(":", 1)[1]
            _poll_and_receipt(base, actor, probe_entity, watch, allow_open=False)

        elif op == "react_loop":
            try:
                result = _react_step(base, goal_id)
                status = result.get("status", "")
                print(f"[runner] react status={status}", flush=True)
                if status == "Succeeded":
                    print(f"[runner] Goal {goal_id[:8]}... Succeeded — exiting", flush=True)
                    break
            except Exception as e:
                print(f"[runner] react error: {e}", flush=True)

    print("[runner] guided exit", flush=True)


def run_continuous(base: str, actor: str, entity: str, watch: str, poll: float, stability_count: int) -> None:
    """Continuous daemon: open → receipt on change → stable → new intent cycle."""
    global _running

    def _stop(*_): global _running; _running = False  # noqa: E702
    signal.signal(signal.SIGTERM, _stop)

    sensor_path = f"runner:{entity}"
    intent_id = _open_intent(base, actor, entity)
    last_hash = _sha256(watch)
    _send_receipt(base, actor, intent_id, last_hash, sensor_path)
    stable = 1
    print(f"[runner] INIT hash={last_hash[:12]}...", flush=True)

    while _running:
        time.sleep(poll)
        try:
            h = _sha256(watch)
        except FileNotFoundError:
            continue

        if h != last_hash:
            print(f"[runner] CHANGE {last_hash[:8]}...→{h[:8]}...", flush=True)
            _send_receipt(base, actor, intent_id, h, sensor_path)
            last_hash = h
            stable = 0
        else:
            stable += 1
            if stable == stability_count:
                print(f"[runner] STABLE — opening new intent cycle", flush=True)
                intent_id = _open_intent(base, actor, entity)
                _send_receipt(base, actor, intent_id, last_hash, sensor_path)
                stable = 0

    print("[runner] Exit", flush=True)


def main() -> None:
    p = argparse.ArgumentParser(description="HipCortex autonomous file runner v3.7.0")
    p.add_argument("--watch", required=True, help="File path to watch")
    p.add_argument("--actor", default="runner-1")
    p.add_argument("--entity", default="filesystem")
    p.add_argument("--server", default="http://localhost:3030")
    p.add_argument("--poll", type=float, default=0.5, help="Poll interval seconds")
    p.add_argument("--stability-count", type=int, default=3, help="Stable polls before new intent cycle")
    p.add_argument("--one-shot", action="store_true",
                   help="Baseline + wait for change + surprising receipt + exit")
    p.add_argument("--guided", action="store_true",
                   help="Scorecard-driven mode: probe entities + react loop until goal Succeeded")
    p.add_argument("--goal-id", default="",
                   help="Goal UUID to drive in --guided mode")
    args = p.parse_args()

    print(f"[runner] Watching {args.watch} actor={args.actor} entity={args.entity} "
          f"one_shot={args.one_shot} guided={args.guided}", flush=True)

    if args.one_shot:
        run_one_shot(args.server, args.actor, args.entity, args.watch, args.poll)
    elif args.guided:
        if not args.goal_id:
            print("[runner] --guided requires --goal-id", file=sys.stderr)
            sys.exit(1)
        run_guided(args.server, args.actor, args.entity, args.watch, args.poll, args.goal_id)
    else:
        run_continuous(args.server, args.actor, args.entity, args.watch, args.poll,
                       args.stability_count)


if __name__ == "__main__":
    main()
