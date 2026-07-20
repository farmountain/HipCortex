#!/usr/bin/env python3
"""Stamp product version strings from the repo-root VERSION file.

Source of truth: ``VERSION`` (one semver line, e.g. ``0.5.0``), kept in
sync with ``Cargo.toml`` ``[package].version``.

Usage
-----
  # Print version from VERSION
  python scripts/stamp_versions.py
  python scripts/stamp_versions.py --print

  # Update MCP serverInfo.version in sdk/mcp/server.py
  python scripts/stamp_versions.py --mcp

  # Dry-run: show what would change, exit 1 if mismatch (CI-friendly)
  python scripts/stamp_versions.py --mcp --check

  # Explicit version override (does not rewrite VERSION)
  python scripts/stamp_versions.py --mcp --version 0.5.0

Examples
--------
  python scripts/stamp_versions.py --mcp
  python scripts/stamp_versions.py --mcp --check
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
VERSION_FILE = REPO_ROOT / "VERSION"
MCP_SERVER = REPO_ROOT / "sdk" / "mcp" / "server.py"
# PyPI package ships a copy for `hipcortex install` when repo path is absent
MCP_BUNDLED = REPO_ROOT / "sdk" / "python" / "hipcortex" / "install" / "mcp_server.py"

# Matches: "version": "x.y.z" inside serverInfo dict (initialize response)
MCP_VERSION_RE = re.compile(
    r'("serverInfo"\s*:\s*\{\s*"name"\s*:\s*"hipcortex"\s*,\s*"version"\s*:\s*")([^"]+)(")'
)


def read_version(path: Path = VERSION_FILE) -> str:
    if not path.is_file():
        raise SystemExit(f"VERSION file missing: {path}")
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        raise SystemExit(f"VERSION file empty: {path}")
    # first non-empty line, strip comments
    line = text.splitlines()[0].split("#", 1)[0].strip()
    if not re.fullmatch(r"\d+\.\d+\.\d+([\w.+-]*)?", line):
        raise SystemExit(f"VERSION not a semver-like string: {line!r}")
    return line


def current_mcp_version(path: Path = MCP_SERVER) -> str | None:
    if not path.is_file():
        raise SystemExit(f"MCP server missing: {path}")
    text = path.read_text(encoding="utf-8")
    m = MCP_VERSION_RE.search(text)
    return m.group(2) if m else None


def sync_mcp_bundle(*, check_only: bool = False) -> int:
    """Keep hipcortex/install/mcp_server.py identical to sdk/mcp/server.py."""
    if not MCP_SERVER.is_file():
        print(f"ERROR: MCP server missing: {MCP_SERVER}", file=sys.stderr)
        return 1
    src = MCP_SERVER.read_text(encoding="utf-8")
    if MCP_BUNDLED.is_file():
        dst = MCP_BUNDLED.read_text(encoding="utf-8")
        if dst == src:
            print(f"MCP bundle already in sync: {MCP_BUNDLED.relative_to(REPO_ROOT)}")
            return 0
        if check_only:
            print(
                f"MISMATCH: {MCP_BUNDLED.relative_to(REPO_ROOT)} differs from sdk/mcp/server.py",
                file=sys.stderr,
            )
            return 1
    elif check_only:
        print(f"MISMATCH: MCP bundle missing: {MCP_BUNDLED}", file=sys.stderr)
        return 1
    MCP_BUNDLED.parent.mkdir(parents=True, exist_ok=True)
    MCP_BUNDLED.write_text(src, encoding="utf-8")
    print(f"Synced MCP bundle → {MCP_BUNDLED.relative_to(REPO_ROOT)}")
    return 0


def stamp_mcp(version: str, *, check_only: bool = False) -> int:
    """Set serverInfo.version in sdk/mcp/server.py. Return 0 ok, 1 mismatch/fail."""
    if not MCP_SERVER.is_file():
        print(f"ERROR: MCP server missing: {MCP_SERVER}", file=sys.stderr)
        return 1
    text = MCP_SERVER.read_text(encoding="utf-8")
    m = MCP_VERSION_RE.search(text)
    if not m:
        print("ERROR: serverInfo version pattern not found in sdk/mcp/server.py", file=sys.stderr)
        return 1
    old = m.group(2)
    rc = 0
    if old == version:
        print(f"MCP serverInfo.version already {version}")
    elif check_only:
        print(f"MISMATCH: MCP serverInfo.version={old!r} != VERSION={version!r}")
        rc = 1
    else:
        new_text = MCP_VERSION_RE.sub(rf"\g<1>{version}\g<3>", text, count=1)
        MCP_SERVER.write_text(new_text, encoding="utf-8")
        print(f"Stamped MCP serverInfo.version: {old} -> {version}")
    # Always keep PyPI-bundled copy aligned (after any stamp write)
    rc = sync_mcp_bundle(check_only=check_only) or rc
    return rc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Stamp product versions from repo-root VERSION file."
    )
    parser.add_argument(
        "--print",
        action="store_true",
        dest="do_print",
        help="Print VERSION and exit (default if no stamp flags).",
    )
    parser.add_argument(
        "--version",
        metavar="SEMVER",
        help="Override version (default: read VERSION file).",
    )
    parser.add_argument(
        "--mcp",
        action="store_true",
        help="Update sdk/mcp/server.py serverInfo.version.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Do not write; exit 1 if stamped targets disagree with VERSION.",
    )
    args = parser.parse_args(argv)

    version = args.version if args.version else read_version()

    # Default action: print version
    if not args.mcp and not args.do_print:
        args.do_print = True

    rc = 0
    if args.do_print:
        print(version)
        if not args.mcp:
            cur = current_mcp_version()
            if cur is not None and cur != version:
                print(
                    f"note: MCP banner is {cur!r} (run with --mcp to stamp)",
                    file=sys.stderr,
                )

    if args.mcp:
        rc = stamp_mcp(version, check_only=args.check) or rc

    return rc


if __name__ == "__main__":
    sys.exit(main())
