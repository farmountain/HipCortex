#!/usr/bin/env python3
"""Latency benchmark: HipCortex REST API vs Mem0 (cloud) vs in-process dict baseline.

Usage
-----
# Minimal (HipCortex only, server must be running):
    cargo run --example mcp_server --no-default-features --features "web-server,petgraph_backend"
    python benchmarks/python_benchmark.py --url http://localhost:3000 -n 200

# Full comparison (requires pip install hipcortex[benchmark] and a MEM0_API_KEY env var):
    MEM0_API_KEY=<key> python benchmarks/python_benchmark.py --url http://localhost:3000 --mem0 -n 200

Output columns
--------------
  backend        | n_ops | add_p50_ms | add_p95_ms | query_p50_ms | query_p95_ms
"""

from __future__ import annotations

import argparse
import os
import statistics
import sys
import time
import uuid
from typing import Callable, Dict, List, Tuple

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _timed_call(fn: Callable, *args, **kwargs) -> float:
    """Return wall-clock seconds for a single fn call."""
    t0 = time.perf_counter()
    fn(*args, **kwargs)
    return time.perf_counter() - t0


def _stats(samples: List[float]) -> Dict[str, float]:
    ms = [s * 1000 for s in samples]
    return {
        "mean": statistics.mean(ms),
        "p50": statistics.median(ms),
        "p95": sorted(ms)[int(len(ms) * 0.95)],
        "min": min(ms),
        "max": max(ms),
    }


def _print_table(rows: List[Dict]) -> None:
    """Pretty-print benchmark results as an ASCII table."""
    cols = ["backend", "n_ops", "add_p50_ms", "add_p95_ms", "query_p50_ms", "query_p95_ms"]
    widths = {c: max(len(c), max(len(str(r.get(c, ""))) for r in rows)) for c in cols}
    sep = "+-" + "-+-".join("-" * widths[c] for c in cols) + "-+"
    header = "| " + " | ".join(c.ljust(widths[c]) for c in cols) + " |"
    print(sep)
    print(header)
    print(sep)
    for row in rows:
        line = "| " + " | ".join(str(row.get(c, "")).ljust(widths[c]) for c in cols) + " |"
        print(line)
    print(sep)


# ---------------------------------------------------------------------------
# Benchmark runners
# ---------------------------------------------------------------------------


def bench_hipcortex(base_url: str, n: int) -> Dict:
    """Benchmark HipCortex REST API add + query operations."""
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))
    from hipcortex import HipCortexClient  # noqa: PLC0415

    client = HipCortexClient(base_url=base_url)
    if not client.health():
        return {"backend": "hipcortex", "error": f"server not reachable at {base_url}"}

    actor = f"bench-{uuid.uuid4().hex[:8]}"
    add_times: List[float] = []
    query_times: List[float] = []

    for i in range(n):
        add_times.append(_timed_call(
            client.add_memory,
            actor=actor,
            action="benchmark_write",
            target=f"observation #{i}",
            record_type="Temporal",
        ))

    for _ in range(n):
        query_times.append(_timed_call(client.query_memory, actor=actor, limit=50))

    # Cleanup
    client.forget(actor)

    add_s = _stats(add_times)
    qry_s = _stats(query_times)
    return {
        "backend": "hipcortex",
        "n_ops": n,
        "add_p50_ms": f"{add_s['p50']:.2f}",
        "add_p95_ms": f"{add_s['p95']:.2f}",
        "query_p50_ms": f"{qry_s['p50']:.2f}",
        "query_p95_ms": f"{qry_s['p95']:.2f}",
    }


def bench_mem0(n: int) -> Dict:
    """Benchmark Mem0 cloud API (requires MEM0_API_KEY env var)."""
    try:
        from mem0 import MemoryClient  # noqa: PLC0415
    except ImportError:
        return {"backend": "mem0", "error": "pip install mem0ai"}

    api_key = os.environ.get("MEM0_API_KEY")
    if not api_key:
        return {"backend": "mem0", "error": "MEM0_API_KEY not set"}

    client = MemoryClient(api_key=api_key)
    user_id = f"bench-{uuid.uuid4().hex[:8]}"
    add_times: List[float] = []
    query_times: List[float] = []

    for i in range(n):
        messages = [{"role": "user", "content": f"observation #{i}"}]
        add_times.append(_timed_call(client.add, messages, user_id=user_id))

    for _ in range(n):
        query_times.append(_timed_call(client.search, "observation", user_id=user_id))

    add_s = _stats(add_times)
    qry_s = _stats(query_times)
    return {
        "backend": "mem0",
        "n_ops": n,
        "add_p50_ms": f"{add_s['p50']:.2f}",
        "add_p95_ms": f"{add_s['p95']:.2f}",
        "query_p50_ms": f"{qry_s['p50']:.2f}",
        "query_p95_ms": f"{qry_s['p95']:.2f}",
    }


def bench_in_process_dict(n: int) -> Dict:
    """Baseline: pure in-process Python dict — shows theoretical minimum latency floor."""
    store: Dict[str, List] = {}
    add_times: List[float] = []
    query_times: List[float] = []
    actor = "bench-local"

    for i in range(n):
        t0 = time.perf_counter()
        store.setdefault(actor, []).append({"content": f"observation #{i}"})
        add_times.append(time.perf_counter() - t0)

    for _ in range(n):
        t0 = time.perf_counter()
        _ = store.get(actor, [])[:50]
        query_times.append(time.perf_counter() - t0)

    add_s = _stats(add_times)
    qry_s = _stats(query_times)
    return {
        "backend": "in-process dict (baseline)",
        "n_ops": n,
        "add_p50_ms": f"{add_s['p50']:.4f}",
        "add_p95_ms": f"{add_s['p95']:.4f}",
        "query_p50_ms": f"{qry_s['p50']:.4f}",
        "query_p95_ms": f"{qry_s['p95']:.4f}",
    }


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(description="HipCortex latency benchmark")
    parser.add_argument(
        "--url", default="http://localhost:3000", help="HipCortex server base URL"
    )
    parser.add_argument(
        "-n", type=int, default=100, help="Number of add/query operations per backend"
    )
    parser.add_argument("--mem0", action="store_true", help="Include Mem0 cloud benchmark")
    parser.add_argument("--no-baseline", action="store_true", help="Skip in-process baseline")
    args = parser.parse_args()

    print(f"\nHipCortex Latency Benchmark  (n={args.n} per backend)\n")

    results: List[Dict] = []

    print(f"  Running HipCortex benchmark against {args.url} ...")
    results.append(bench_hipcortex(args.url, args.n))

    if args.mem0:
        print("  Running Mem0 benchmark ...")
        results.append(bench_mem0(args.n))

    if not args.no_baseline:
        print("  Running in-process dict baseline ...")
        results.append(bench_in_process_dict(args.n))

    print()
    _print_table(results)
    print()


if __name__ == "__main__":
    main()
