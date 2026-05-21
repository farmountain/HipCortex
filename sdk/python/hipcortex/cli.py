"""HipCortex CLI — install command auto-configures AI coding assistants.

Usage:
    hipcortex install           # download binary + configure Claude Code + Cursor
    hipcortex install --url URL # use existing server instead of local binary
    hipcortex start             # start the local server (downloads if needed)
    hipcortex status            # check server health
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import stat
import sys
import urllib.request
from pathlib import Path
from typing import Optional

# ─── Constants ───────────────────────────────────────────────────────────────

GITHUB_RELEASES = "https://github.com/farmountain/HipCortex/releases/latest/download"
INSTALL_DIR = Path.home() / ".hipcortex"
BINARY_NAME = "hipcortex-server"
DEFAULT_URL = "http://localhost:3030"
MANAGED_URL = "https://hipcortex.fly.dev"

# Claude Code registration line appended to ~/.claude/CLAUDE.md
_CLAUDE_REGISTRATION = """
# hipcortex
- **hipcortex** (`~/.claude/skills/hipcortex/SKILL.md`) - Persistent memory for AI agents. Store decisions, recall context, GDPR forget. Trigger: `/hipcortex`
When the user types `/hipcortex`, invoke the Skill tool with `skill: "hipcortex"` before doing anything else.
"""

# ─── Platform detection ───────────────────────────────────────────────────────

def _detect_platform() -> tuple[str, str]:
    """Return (os_name, arch) matching GitHub release asset names."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "darwin":
        os_name = "macos"
    elif system == "linux":
        os_name = "linux"
    elif system == "windows":
        os_name = "windows"
    else:
        raise RuntimeError(f"Unsupported OS: {system}")

    if machine in ("arm64", "aarch64"):
        arch = "arm64"
    elif machine in ("x86_64", "amd64"):
        arch = "amd64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return os_name, arch


def _binary_url(os_name: str, arch: str) -> str:
    name = f"hipcortex-{os_name}-{arch}"
    if os_name == "windows":
        name += ".exe"
    return f"{GITHUB_RELEASES}/{name}"


def _binary_path(os_name: str, arch: str) -> Path:
    name = BINARY_NAME
    if os_name == "windows":
        name += ".exe"
    return INSTALL_DIR / name

# ─── Download ────────────────────────────────────────────────────────────────

def _download_binary(url: str, dest: Path) -> None:
    """Download binary with progress indicator."""
    INSTALL_DIR.mkdir(parents=True, exist_ok=True)
    print(f"  Downloading {url.split('/')[-1]} ...", end=" ", flush=True)
    try:
        urllib.request.urlretrieve(url, str(dest))
        dest.chmod(dest.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)
        print("✓")
    except urllib.error.HTTPError as e:
        print(f"✗ (HTTP {e.code})")
        raise RuntimeError(
            f"Could not download binary from {url}\n"
            "Check https://github.com/farmountain/HipCortex/releases for available builds."
        ) from e
    except Exception as e:
        print(f"✗ ({e})")
        raise

# ─── Skill registration (Claude Code) ────────────────────────────────────────

def _skill_dir() -> Path:
    return Path.home() / ".claude" / "skills" / "hipcortex"


def _install_claude_code(server_url: str) -> bool:
    """Write SKILL.md + append to CLAUDE.md. Returns True on success."""
    claude_dir = Path.home() / ".claude"
    if not claude_dir.exists():
        return False  # Claude Code not installed

    # Write skill file
    skill_dir = _skill_dir()
    skill_dir.mkdir(parents=True, exist_ok=True)
    skill_md_src = Path(__file__).parent / "install" / "SKILL.md"

    # Read template and substitute server URL
    content = skill_md_src.read_text(encoding="utf-8")
    content = content.replace("http://localhost:3030", server_url)
    (skill_dir / "SKILL.md").write_text(content, encoding="utf-8")

    # Append registration to CLAUDE.md if not already present
    claude_md = claude_dir / "CLAUDE.md"
    existing = claude_md.read_text(encoding="utf-8") if claude_md.exists() else ""
    if "hipcortex" not in existing:
        with claude_md.open("a", encoding="utf-8") as f:
            f.write(_CLAUDE_REGISTRATION)

    return True


def _uninstall_claude_code() -> None:
    """Remove HipCortex skill from Claude Code."""
    skill_dir = _skill_dir()
    if skill_dir.exists():
        shutil.rmtree(skill_dir)
    claude_md = Path.home() / ".claude" / "CLAUDE.md"
    if claude_md.exists():
        text = claude_md.read_text(encoding="utf-8")
        # Remove the registration block
        marker = "\n# hipcortex"
        if marker in text:
            idx = text.index(marker)
            # Find end of registration block (next \n# or end of file)
            end = text.find("\n# ", idx + 1)
            claude_md.write_text(
                text[:idx] + (text[end:] if end != -1 else ""),
                encoding="utf-8",
            )

# ─── MCP registration (Cursor / Windsurf) ────────────────────────────────────

def _cursor_mcp_path(global_: bool = False) -> Optional[Path]:
    """Return path to Cursor mcp.json — local (project) or global."""
    if global_:
        # Global Cursor config location by OS
        if platform.system() == "Windows":
            base = Path(os.environ.get("APPDATA", Path.home()))
        elif platform.system() == "Darwin":
            base = Path.home() / "Library" / "Application Support" / "Cursor"
        else:
            base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "Cursor"
        mcp_path = base / "mcp.json"
        return mcp_path
    else:
        return Path.cwd() / ".cursor" / "mcp.json"


def _install_cursor(server_url: str, global_: bool = False) -> bool:
    """Write/update .cursor/mcp.json. Returns True on success."""
    mcp_path = _cursor_mcp_path(global_=global_)
    if mcp_path is None:
        return False

    mcp_path.parent.mkdir(parents=True, exist_ok=True)

    # Read existing config or start fresh
    existing: dict = {}
    if mcp_path.exists():
        try:
            existing = json.loads(mcp_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            existing = {}

    # Inject hipcortex MCP server entry
    mcp_servers = existing.setdefault("mcpServers", {})
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    mcp_servers["hipcortex"] = {
        "command": sys.executable,
        "args": [mcp_server_py],
        "env": {"HIPCORTEX_URL": server_url},
    }

    mcp_path.write_text(json.dumps(existing, indent=2), encoding="utf-8")
    return True


def _install_vscode(server_url: str) -> bool:
    """Write mcpServers to VS Code settings.json. Returns True on success."""
    if platform.system() == "Windows":
        settings_path = Path(os.environ.get("APPDATA", "")) / "Code" / "User" / "settings.json"
    elif platform.system() == "Darwin":
        settings_path = Path.home() / "Library" / "Application Support" / "Code" / "User" / "settings.json"
    else:
        settings_path = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "Code" / "User" / "settings.json"

    if not settings_path.exists():
        return False  # VS Code not installed

    try:
        settings: dict = json.loads(settings_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, PermissionError):
        return False

    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    mcp_servers = settings.setdefault("mcpServers", {})
    mcp_servers["hipcortex"] = {
        "command": sys.executable,
        "args": [mcp_server_py],
        "env": {"HIPCORTEX_URL": server_url},
    }
    settings_path.write_text(json.dumps(settings, indent=2), encoding="utf-8")
    return True

# ─── MCP server script installer ─────────────────────────────────────────────

def _install_mcp_server() -> None:
    """Copy the MCP server script to ~/.hipcortex-mcp/server.py."""
    mcp_dir = Path.home() / ".hipcortex-mcp"
    mcp_dir.mkdir(parents=True, exist_ok=True)
    src = Path(__file__).parent.parent.parent / "mcp" / "server.py"  # sdk/mcp/server.py
    # Try repo-relative path first, then PyPI-installed package
    if not src.exists():
        import importlib.resources
        try:
            # When installed as a package, look for bundled resource
            with importlib.resources.path("hipcortex.install", "mcp_server.py") as p:
                src = p
        except Exception:
            return  # MCP server not bundled, skip

    if src.exists():
        shutil.copy2(str(src), str(mcp_dir / "server.py"))

# ─── Commands ────────────────────────────────────────────────────────────────

def cmd_install(args: argparse.Namespace) -> None:
    """Download binary + configure all detected AI coding assistants."""
    print("\nHipCortex installer\n" + "=" * 40)

    # Determine server URL
    if args.url:
        server_url = args.url.rstrip("/")
        binary_path = None
        print(f"Using existing server: {server_url}")
    else:
        os_name, arch = _detect_platform()
        binary_path = _binary_path(os_name, arch)
        server_url = DEFAULT_URL

        if binary_path.exists() and not args.force:
            print(f"  Binary already at {binary_path} (use --force to re-download)")
        else:
            url = _binary_url(os_name, arch)
            _download_binary(url, binary_path)

    # Install MCP server script for Cursor/VS Code
    _install_mcp_server()

    # Register with AI coding assistants
    print("\nRegistering with AI coding assistants:")
    results = []

    if _install_claude_code(server_url):
        results.append(("Claude Code", "✓", f"~/.claude/skills/hipcortex/"))
    else:
        results.append(("Claude Code", "–", "not found (install from claude.ai/code)"))

    if _install_cursor(server_url, global_=False):
        results.append(("Cursor (project)", "✓", str(Path.cwd() / ".cursor" / "mcp.json")))
    elif _install_cursor(server_url, global_=True):
        mcp_path = _cursor_mcp_path(global_=True)
        results.append(("Cursor (global)", "✓", str(mcp_path)))
    else:
        results.append(("Cursor", "–", "not found"))

    if _install_vscode(server_url):
        results.append(("VS Code", "✓", "settings.json"))
    else:
        results.append(("VS Code", "–", "not found"))

    for name, status, detail in results:
        print(f"  {status} {name:<20} {detail}")

    print()

    if binary_path:
        print(f"Binary: {binary_path}")
        print(f"Start:  hipcortex start")
        print()

    # Print usage instructions
    claude_ok = any(s == "✓" and "Claude Code" in n for n, s, _ in results)
    cursor_ok = any(s == "✓" and "Cursor" in n for n, s, _ in results)

    if claude_ok:
        print("Claude Code: type /hipcortex remember 'your note'")
    if cursor_ok:
        print("Cursor: restart and use the hipcortex MCP tools")
    print()
    print(f"Docs: https://github.com/farmountain/HipCortex")


def cmd_start(args: argparse.Namespace) -> None:
    """Start the local HipCortex server."""
    try:
        os_name, arch = _detect_platform()
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    binary = _binary_path(os_name, arch)
    if not binary.exists():
        print(f"Binary not found at {binary}")
        print("Run: hipcortex install")
        sys.exit(1)

    port = args.port or int(os.environ.get("PORT", "3030"))
    data_dir = args.data_dir or os.environ.get("DATA_DIR", str(Path.home() / ".hipcortex" / "data"))
    Path(data_dir).mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["PORT"] = str(port)
    env["DATA_DIR"] = data_dir

    print(f"Starting HipCortex on http://localhost:{port}")
    print(f"Data: {data_dir}")
    print("Ctrl+C to stop\n")

    import subprocess
    proc = subprocess.Popen([str(binary)], env=env)
    try:
        proc.wait()
    except KeyboardInterrupt:
        proc.terminate()
        proc.wait()


def cmd_status(args: argparse.Namespace) -> None:
    """Check server health."""
    url = args.url or os.environ.get("HIPCORTEX_URL", DEFAULT_URL)
    try:
        import urllib.request
        with urllib.request.urlopen(f"{url}/health", timeout=5) as r:
            print(f"✓ HipCortex running at {url} ({r.read().decode().strip()})")
    except Exception as e:
        print(f"✗ Not reachable at {url}: {e}")
        print("  Run: hipcortex start")


def cmd_uninstall(args: argparse.Namespace) -> None:
    """Remove HipCortex from AI coding assistants and optionally delete binary."""
    print("Uninstalling HipCortex...")
    _uninstall_claude_code()
    print("  ✓ Removed from Claude Code")

    if args.purge:
        if INSTALL_DIR.exists():
            shutil.rmtree(INSTALL_DIR)
            print(f"  ✓ Deleted {INSTALL_DIR}")
        mcp_dir = Path.home() / ".hipcortex-mcp"
        if mcp_dir.exists():
            shutil.rmtree(mcp_dir)
            print(f"  ✓ Deleted {mcp_dir}")

# ─── Argument parser ──────────────────────────────────────────────────────────

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="hipcortex",
        description="HipCortex memory engine — persistent causal memory for AI agents",
    )
    sub = parser.add_subparsers(dest="command")

    # install
    p_install = sub.add_parser("install", help="Download binary + configure AI coding assistants")
    p_install.add_argument("--url", help=f"Use an existing server instead of local binary (e.g. {MANAGED_URL})")
    p_install.add_argument("--force", action="store_true", help="Re-download binary even if it exists")

    # start
    p_start = sub.add_parser("start", help="Start the local HipCortex server")
    p_start.add_argument("--port", type=int, help="Port (default: 3030)")
    p_start.add_argument("--data-dir", help="Data directory (default: ~/.hipcortex/data)")

    # status
    p_status = sub.add_parser("status", help="Check server health")
    p_status.add_argument("--url", help="Server URL to check")

    # uninstall
    p_uninstall = sub.add_parser("uninstall", help="Remove HipCortex configuration")
    p_uninstall.add_argument("--purge", action="store_true", help="Also delete downloaded binary and data")

    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    if args.command == "install":
        cmd_install(args)
    elif args.command == "start":
        cmd_start(args)
    elif args.command == "status":
        cmd_status(args)
    elif args.command == "uninstall":
        cmd_uninstall(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
