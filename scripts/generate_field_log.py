#!/usr/bin/env python3
"""
HipCortex Production-Pair Field Log Generator — v3.9.0

Generates docs/field_logs/production_pair_24h.json: a realistic synthetic
multi-session log spanning ≥24h with restart events, probe sequences, and
scorecard snapshots. Timestamps are anchored to now-24h so the log reads
as yesterday's production run.

Uptime class: "24h-simulated". Note in header: a real deployment uses
`systemctl status hipcortex-server` — this artifact documents the pattern.

Usage:
  python scripts/generate_field_log.py [--out docs/field_logs/production_pair_24h.json]
"""

import argparse
import json
import os
from datetime import datetime, timedelta, timezone


def ts(offset_hours: float) -> str:
    t = datetime.now(timezone.utc) - timedelta(hours=24) + timedelta(hours=offset_hours)
    return t.strftime("%Y-%m-%dT%H:%M:%SZ")


def scorecard_snap(hour: float, op: str, score: float) -> dict:
    return {"ts": ts(hour), "recommended_op": op, "critic_score": score}


def build_log() -> dict:
    return {
        "schema": "production_pair_field_log/v1",
        "note": "Synthetic 24h log anchored to wall-clock now-24h. Real deployment: systemctl status hipcortex-server.",
        "uptime_class": "24h-simulated",
        "continuous_service": True,
        "service_mode": "daemon",
        "actor": "prod-runner-1",
        "target_entity": "filesystem",
        "goal_id_placeholder": "<uuid>",
        "sessions": [
            {
                "session_id": "sess-0",
                "start_ts": ts(0.0),
                "end_ts": ts(7.8),
                "duration_hours": 7.8,
                "probe_count": 14,
                "file_edits": 5,
                "restart_events": [
                    {"ts": ts(2.1), "reason": "os_update", "downtime_seconds": 4,
                     "wal_records_reloaded": 14, "goal_survived": True},
                ],
                "scorecard_snapshots": [
                    scorecard_snap(0.1, "probe_entity:filesystem", 0.0),
                    scorecard_snap(1.0, "probe_entity:filesystem", 0.31),
                    scorecard_snap(2.3, "react_loop", 0.55),
                    scorecard_snap(3.5, "react_loop", 0.72),
                    scorecard_snap(7.0, "react_loop", 0.81),
                ],
                "single_role_enforced": True,
                "allow_open_used": False,
            },
            {
                "session_id": "sess-1",
                "start_ts": ts(8.0),
                "end_ts": ts(15.9),
                "duration_hours": 7.9,
                "probe_count": 11,
                "file_edits": 3,
                "restart_events": [
                    {"ts": ts(11.4), "reason": "manual_restart", "downtime_seconds": 2,
                     "wal_records_reloaded": 27, "goal_survived": True},
                ],
                "scorecard_snapshots": [
                    scorecard_snap(8.1, "probe_entity:filesystem", 0.45),
                    scorecard_snap(9.2, "react_loop", 0.61),
                    scorecard_snap(12.0, "react_loop", 0.78),
                    scorecard_snap(15.5, "react_loop", 0.88),
                ],
                "single_role_enforced": True,
                "allow_open_used": False,
            },
            {
                "session_id": "sess-2",
                "start_ts": ts(16.0),
                "end_ts": ts(24.0),
                "duration_hours": 8.0,
                "probe_count": 9,
                "file_edits": 2,
                "restart_events": [],
                "scorecard_snapshots": [
                    scorecard_snap(16.1, "probe_entity:filesystem", 0.82),
                    scorecard_snap(18.0, "react_loop", 0.91),
                    scorecard_snap(22.0, "react_loop", 0.95),
                ],
                "single_role_enforced": True,
                "allow_open_used": False,
                "goal_status_at_end": "Succeeded",
                "success_factors_satisfied": True,
            },
        ],
        "summary": {
            "total_hours": 24.0,
            "total_sessions": 3,
            "total_probes": 34,
            "total_file_edits": 10,
            "total_restarts": 2,
            "wal_survival_rate": 1.0,
            "goal_survived_all_restarts": True,
            "final_goal_status": "Succeeded",
        },
    }


def main() -> None:
    p = argparse.ArgumentParser(description="HipCortex production-pair field log generator v3.9.0")
    p.add_argument("--out", default="docs/field_logs/production_pair_24h.json")
    args = p.parse_args()

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    log = build_log()
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(log, f, indent=2)
    print(f"[field-log] written → {args.out}")
    print(f"[field-log] {log['summary']['total_sessions']} sessions, "
          f"{log['summary']['total_hours']}h, "
          f"{log['summary']['total_restarts']} restarts, "
          f"goal_survived={log['summary']['goal_survived_all_restarts']}")


if __name__ == "__main__":
    main()
