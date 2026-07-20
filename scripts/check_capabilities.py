#!/usr/bin/env python3
"""Verify docs/capabilities.md stays honest vs MCP TOOLS (+ optional SDK scans).

Usage:
    python scripts/check_capabilities.py           # full MCP↔doc check (default)
    python scripts/check_capabilities.py --check-mcp  # list MCP tools + exit 0 if non-empty
    python scripts/check_capabilities.py --list-sdk   # print Python/TS method names

Exit codes:
    0  OK
    1  MCP tool missing from docs/capabilities.md, empty TOOLS, or missing version
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MCP_SERVER = ROOT / "sdk" / "mcp" / "server.py"
CAPABILITIES = ROOT / "docs" / "capabilities.md"
PY_CLIENT = ROOT / "sdk" / "python" / "hipcortex" / "client.py"
TS_CLIENT = ROOT / "sdk" / "typescript" / "src" / "client.ts"
VERSION_FILE = ROOT / "VERSION"


def expected_mcp_version() -> str:
    """Product version from VERSION file (stamped into MCP serverInfo)."""
    if not VERSION_FILE.is_file():
        return "0.0.0"
    line = VERSION_FILE.read_text(encoding="utf-8").splitlines()[0].split("#", 1)[0].strip()
    return line or "0.0.0"


def extract_mcp_tools(source: str) -> list[str]:
    """Parse tool names from TOOLS = [ { "name": "..." }, ... ]."""
    # Prefer slice from TOOLS = [ ... ] to reduce false positives outside the list.
    m = re.search(r"\bTOOLS\s*=\s*\[", source)
    if not m:
        return []
    start = m.end() - 1  # at '['
    depth = 0
    end = None
    for i, ch in enumerate(source[start:], start):
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth -= 1
            if depth == 0:
                end = i
                break
    block = source[start : end + 1] if end is not None else source[m.start() :]
    names = re.findall(r'["\']name["\']\s*:\s*["\']([a-zA-Z_][a-zA-Z0-9_]*)["\']', block)
    # Keep order, unique
    seen: set[str] = set()
    out: list[str] = []
    for n in names:
        if n not in seen:
            seen.add(n)
            out.append(n)
    return out


def extract_mcp_version(source: str) -> str | None:
    m = re.search(
        r'["\']version["\']\s*:\s*["\']([0-9]+\.[0-9]+\.[0-9]+)["\']',
        source,
    )
    return m.group(1) if m else None


def extract_py_methods(source: str) -> list[str]:
    return re.findall(r"^\s+def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(", source, re.M)


def extract_ts_methods(source: str) -> list[str]:
    # async methodName( or methodName(
    return re.findall(
        r"^\s+(?:async\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*(?::\s*[^{]+)?\s*\{",
        source,
        re.M,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check-mcp",
        action="store_true",
        help="List MCP tools; exit 0 if non-empty and version present",
    )
    parser.add_argument(
        "--list-sdk",
        action="store_true",
        help="Print Python client + TS client method names",
    )
    args = parser.parse_args()

    if not MCP_SERVER.is_file():
        print(f"ERROR: missing {MCP_SERVER}", file=sys.stderr)
        return 1

    mcp_src = MCP_SERVER.read_text(encoding="utf-8")
    tools = extract_mcp_tools(mcp_src)
    version = extract_mcp_version(mcp_src)

    if args.list_sdk:
        if PY_CLIENT.is_file():
            py = extract_py_methods(PY_CLIENT.read_text(encoding="utf-8"))
            print("Python HipCortexClient methods:")
            for n in py:
                if not n.startswith("_"):
                    print(f"  {n}")
        if TS_CLIENT.is_file():
            ts = extract_ts_methods(TS_CLIENT.read_text(encoding="utf-8"))
            skip = {"constructor", "request"}
            print("TS HipCortexClient methods:")
            for n in ts:
                if n not in skip and not n.startswith("_"):
                    print(f"  {n}")
        return 0

    if args.check_mcp:
        print(f"MCP server: {MCP_SERVER.relative_to(ROOT)}")
        print(f"version: {version or '(missing)'}")
        print(f"TOOLS ({len(tools)}):")
        for t in tools:
            print(f"  - {t}")
        if not tools:
            print("ERROR: TOOLS list empty", file=sys.stderr)
            return 1
        want = expected_mcp_version()
        if version != want:
            print(
                f"ERROR: expected MCP version {want}, got {version!r}",
                file=sys.stderr,
            )
            return 1
        print("OK: MCP tools non-empty, version present")
        return 0

    # Default: every MCP tool name must appear in capabilities.md
    if not CAPABILITIES.is_file():
        print(f"ERROR: missing {CAPABILITIES}", file=sys.stderr)
        return 1
    if not tools:
        print("ERROR: no MCP TOOLS names parsed from server.py", file=sys.stderr)
        return 1
    want = expected_mcp_version()
    if version != want:
        print(
            f"ERROR: expected MCP version {want} in server.py, got {version!r}",
            file=sys.stderr,
        )
        return 1

    doc = CAPABILITIES.read_text(encoding="utf-8")
    missing = [t for t in tools if t not in doc]
    if missing:
        print("ERROR: MCP tools missing from docs/capabilities.md:", file=sys.stderr)
        for t in missing:
            print(f"  - {t}", file=sys.stderr)
        return 1

    # Soft: if doc claims MCP Y for reflect, tool must exist (guards post-4B honesty)
    if re.search(r"\|\s*reflect\s*\|[^|]*\|\s*Y\s*\|", doc, re.I):
        if "reflect" not in tools:
            print(
                "ERROR: matrix claims reflect MCP=Y but reflect not in TOOLS",
                file=sys.stderr,
            )
            return 1

    print(f"OK: {len(tools)} MCP tools documented in capabilities.md (v{version})")
    for t in tools:
        print(f"  ✓ {t}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
