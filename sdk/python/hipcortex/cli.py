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

# ─── Windsurf MCP registration ────────────────────────────────────────────────

def _install_windsurf(server_url: str) -> bool:
    """Write/update Windsurf MCP config."""
    if platform.system() == "Windows":
        base = Path(os.environ.get("APPDATA", Path.home())) / "Codeium" / "windsurf"
    elif platform.system() == "Darwin":
        base = Path.home() / "Library" / "Application Support" / "Codeium" / "windsurf"
    else:
        base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "Codeium" / "windsurf"

    if not base.exists():
        return False

    mcp_path = base / "mcp_settings.json"
    mcp_path.parent.mkdir(parents=True, exist_ok=True)
    existing: dict = {}
    if mcp_path.exists():
        try:
            existing = json.loads(mcp_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            existing = {}

    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    existing.setdefault("mcpServers", {})["hipcortex"] = {
        "command": sys.executable,
        "args": [mcp_server_py],
        "env": {"HIPCORTEX_URL": server_url},
    }
    mcp_path.write_text(json.dumps(existing, indent=2), encoding="utf-8")
    return True


def _install_mcp_generic(server_url: str, mcp_path: Path) -> bool:
    """Write MCP config to an arbitrary path (Cline, RooCode, Kilo Code, etc.)."""
    mcp_path.parent.mkdir(parents=True, exist_ok=True)
    existing: dict = {}
    if mcp_path.exists():
        try:
            existing = json.loads(mcp_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            existing = {}
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    existing.setdefault("mcpServers", {})["hipcortex"] = {
        "command": sys.executable,
        "args": [mcp_server_py],
        "env": {"HIPCORTEX_URL": server_url},
    }
    mcp_path.write_text(json.dumps(existing, indent=2), encoding="utf-8")
    return True


# ─── Interactive multi-select wizard ─────────────────────────────────────────

_CYAN  = "\033[96m"
_GRAY  = "\033[90m"
_BOLD  = "\033[1m"
_DIM   = "\033[2m"
_GREEN = "\033[92m"
_RESET = "\033[0m"
_CLEAR_LINE = "\033[2K\r"

_SPLASH = f"""{_CYAN}{_BOLD}
  ██╗  ██╗██╗██████╗  ██████╗ ██████╗ ██████╗ ████████╗███████╗██╗  ██╗
  ██║  ██║██║██╔══██╗██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██╔════╝╚██╗██╔╝
  ███████║██║██████╔╝██║     ██║   ██║██████╔╝   ██║   █████╗   ╚███╔╝
  ██╔══██║██║██╔═══╝ ██║     ██║   ██║██╔══██╗   ██║   ██╔══╝   ██╔██╗
  ██║  ██║██║██║     ╚██████╗╚██████╔╝██║  ██║   ██║   ███████╗██╔╝ ██╗
  ╚═╝  ╚═╝╚═╝╚═╝      ╚═════╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝  ╚═╝
{_RESET}  Persistent causal memory for AI agents · {_CYAN}hipcortex.fly.dev{_RESET}
"""

# Agent registry — id, display name, short description, type, install fn / guide URL
# type: "native" | "mcp" | "guide"
def _build_agent_registry(server_url: str, python_exe: str) -> list:
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    mcp_entry = {"command": python_exe, "args": [mcp_server_py], "env": {"HIPCORTEX_URL": server_url}}

    def mcp_project(name: str, rel_path: str):
        p = Path.cwd() / rel_path
        def fn():
            return _install_mcp_generic(server_url, p), str(p)
        return fn

    return [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "Anthropic · SKILL.md native, no MCP process",
            "type": "native",
            "fn": lambda: (_install_claude_code(server_url), "~/.claude/skills/hipcortex/"),
        },
        {
            "id": "cursor",
            "name": "Cursor",
            "desc": "Anysphere · MCP tools in AI panel",
            "type": "mcp",
            "fn": lambda: (
                _install_cursor(server_url, global_=False) or _install_cursor(server_url, global_=True),
                ".cursor/mcp.json"
            ),
        },
        {
            "id": "windsurf",
            "name": "Windsurf",
            "desc": "Codeium · global MCP settings",
            "type": "mcp",
            "fn": lambda: (_install_windsurf(server_url), "~/.codeium/windsurf/mcp_settings.json"),
        },
        {
            "id": "vscode",
            "name": "VS Code",
            "desc": "Microsoft · MCP via settings.json",
            "type": "mcp",
            "fn": lambda: (_install_vscode(server_url), "settings.json"),
        },
        {
            "id": "cline",
            "name": "Cline",
            "desc": "saoudrizwan · .cline/mcp.json in project",
            "type": "mcp",
            "fn": lambda: (_install_mcp_generic(server_url, Path.cwd() / ".cline" / "mcp.json"), ".cline/mcp.json"),
        },
        {
            "id": "roocode",
            "name": "RooCode",
            "desc": "RooVeterinary · .roo/mcp.json in project",
            "type": "mcp",
            "fn": lambda: (_install_mcp_generic(server_url, Path.cwd() / ".roo" / "mcp.json"), ".roo/mcp.json"),
        },
        {
            "id": "continue",
            "name": "Continue",
            "desc": "continuedev · context provider + /remember /recall",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex/blob/main/sdk/continue/README.md",
        },
        {
            "id": "copilot",
            "name": "GitHub Copilot",
            "desc": "GitHub · OpenAPI tool registration",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex#github-copilot",
        },
        {
            "id": "codex",
            "name": "OpenAI Codex CLI",
            "desc": "OpenAI terminal agent · hc-remember shell wrapper",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex/blob/main/sdk/mcp/README.md#shell-integration",
        },
        {
            "id": "aider",
            "name": "Aider",
            "desc": "paul-gauthier · hc-remember shell wrapper",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex/blob/main/sdk/mcp/README.md#shell-integration",
        },
        {
            "id": "gemini",
            "name": "Gemini CLI",
            "desc": "Google · MCP server (gemini_mcp_config)",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex#gemini-cli",
        },
        {
            "id": "amazonq",
            "name": "Amazon Q Developer",
            "desc": "AWS · MCP tool integration",
            "type": "guide",
            "guide": "https://github.com/farmountain/HipCortex#amazon-q",
        },
    ]


def _getch() -> str:
    """Read a single raw keypress. Returns: 'up' | 'down' | 'space' | 'enter' | 'quit'."""
    if platform.system() == "Windows":
        import msvcrt
        ch = msvcrt.getch()
        if ch in (b"\x00", b"\xe0"):
            ch2 = msvcrt.getch()
            return {"H": "up", "P": "down"}.get(chr(ch2[0]), "")
        if ch == b"\r":
            return "enter"
        if ch == b" ":
            return "space"
        if ch in (b"q", b"Q", b"\x03"):
            return "quit"
        return ""
    else:
        import tty, termios
        fd = sys.stdin.fileno()
        old = termios.tcgetattr(fd)
        try:
            tty.setraw(fd)
            ch = sys.stdin.read(1)
            if ch == "\x1b":
                ch2 = sys.stdin.read(1)
                if ch2 == "[":
                    ch3 = sys.stdin.read(1)
                    return {"A": "up", "B": "down"}.get(ch3, "")
                return ""
            if ch in ("\r", "\n"):
                return "enter"
            if ch == " ":
                return "space"
            if ch in ("q", "Q", "\x03"):
                return "quit"
            return ""
        finally:
            termios.tcsetattr(fd, termios.TCSADRAIN, old)


def _run_wizard(agents: list) -> list:
    """Interactive multi-select. Returns list of selected agent dicts."""
    selected = set()
    cursor = 0
    n = len(agents)

    def render():
        # Move cursor to start of list area
        print(f"\033[{n + 3}A", end="")  # move up past list + header + footer
        print(f"  {_BOLD}Select agents to configure:{_RESET} {_GRAY}(Space toggle · Enter confirm · q quit){_RESET}")
        print()
        for i, agent in enumerate(agents):
            is_sel = i in selected
            is_cur = i == cursor
            bullet = f"{_CYAN}●{_RESET}" if is_sel else f"{_GRAY}○{_RESET}"
            name = f"{_BOLD}{agent['name']}{_RESET}" if is_cur else agent["name"]
            desc = f"{_DIM}{agent['desc']}{_RESET}"
            tag = f" {_CYAN}[guide]{_RESET}" if agent["type"] == "guide" else ""
            prefix = "  ›" if is_cur else "   "
            print(f"{prefix} {bullet} {name:<18} {desc}{tag}")
        count = len(selected)
        print(f"\n  {_GREEN}{count} selected{_RESET}" if count else f"\n  {_GRAY}none selected{_RESET}")

    # Initial draw (reserve lines)
    print(f"  {_BOLD}Select agents to configure:{_RESET}")
    print()
    for agent in agents:
        print(f"   ○ {agent['name']:<18} {_DIM}{agent['desc']}{_RESET}")
    print()

    render()

    while True:
        try:
            key = _getch()
        except Exception:
            break

        if key == "quit":
            break
        elif key == "up":
            cursor = (cursor - 1) % n
        elif key == "down":
            cursor = (cursor + 1) % n
        elif key == "space":
            if cursor in selected:
                selected.discard(cursor)
            else:
                selected.add(cursor)
        elif key == "enter":
            break

        render()

    print()  # newline after list
    return [agents[i] for i in sorted(selected)]


# ─── Commands ────────────────────────────────────────────────────────────────

def cmd_install(args: argparse.Namespace) -> None:
    """Interactive wizard: download binary + configure chosen AI coding assistants."""
    print(_SPLASH)

    # ── 1. Server URL ─────────────────────────────────────────────────────────
    if args.url:
        server_url = args.url.rstrip("/")
        binary_path = None
        print(f"  Using server: {_CYAN}{server_url}{_RESET}\n")
    else:
        os_name, arch = _detect_platform()
        binary_path = _binary_path(os_name, arch)
        server_url = DEFAULT_URL

        if binary_path.exists() and not getattr(args, "force", False):
            print(f"  {_GREEN}✓{_RESET} Binary: {binary_path}")
        else:
            print(f"  Downloading binary for {os_name}/{arch}...")
            _download_binary(_binary_url(os_name, arch), binary_path)
            print(f"  {_GREEN}✓{_RESET} Binary: {binary_path}")

    # ── 2. MCP server script ──────────────────────────────────────────────────
    _install_mcp_server()

    # ── 3. Agent selection ────────────────────────────────────────────────────
    agents = _build_agent_registry(server_url, sys.executable)

    yes_all = getattr(args, "yes", False)
    if yes_all:
        # Non-interactive: configure all auto-configurable agents
        chosen = [a for a in agents if a["type"] != "guide"]
        print(f"  --yes: configuring all {len(chosen)} supported agents\n")
    else:
        print(f"  {_BOLD}Which AI coding assistants do you use?{_RESET}\n")
        # Check if terminal is interactive
        if not sys.stdin.isatty():
            chosen = [a for a in agents if a["type"] != "guide"]
            print(f"  Non-interactive: configuring all supported agents\n")
        else:
            chosen = _run_wizard(agents)

    if not chosen:
        print(f"  {_GRAY}Nothing selected. Run 'hipcortex install' again anytime.{_RESET}\n")
        return

    # ── 4. Configure chosen agents ────────────────────────────────────────────
    print(f"\n  {_BOLD}Configuring:{_RESET}\n")
    guide_items = []

    for agent in chosen:
        name = agent["name"]
        if agent["type"] == "guide":
            guide_items.append(agent)
            print(f"  {_CYAN}ℹ{_RESET} {name:<18} guide → {agent['guide']}")
            continue
        try:
            ok, detail = agent["fn"]()
            if ok:
                print(f"  {_GREEN}✓{_RESET} {name:<18} {_DIM}{detail}{_RESET}")
            else:
                print(f"  {_GRAY}–{_RESET} {name:<18} {_DIM}not found (install first){_RESET}")
        except Exception as e:
            print(f"  ✗ {name:<18} error: {e}")

    # ── 5. Start server ───────────────────────────────────────────────────────
    if binary_path and binary_path.exists():
        health_url = "http://localhost:3030/health"
        already_running = False
        try:
            with urllib.request.urlopen(health_url, timeout=1) as r:
                already_running = r.status == 200
        except Exception:
            pass

        if not already_running:
            import subprocess as _sp
            data_dir = str(INSTALL_DIR / "data")
            Path(data_dir).mkdir(parents=True, exist_ok=True)
            env = os.environ.copy()
            env.update({"PORT": "3030", "DATA_DIR": data_dir, "RUST_LOG": "warn"})
            _sp.Popen([str(binary_path)], env=env, stdout=_sp.DEVNULL, stderr=_sp.DEVNULL)
            import time as _t
            print(f"\n  Starting server...", end=" ", flush=True)
            for _ in range(20):
                _t.sleep(0.5)
                try:
                    with urllib.request.urlopen(health_url, timeout=1) as r:
                        if r.status == 200:
                            print(f"{_GREEN}✓{_RESET} running on :3030")
                            break
                except Exception:
                    pass
            else:
                print("(starting in background)")

    # ── 6. Usage hints ────────────────────────────────────────────────────────
    print(f"\n  {_BOLD}Ready!{_RESET}\n")
    configured_names = {a["id"] for a in chosen if a["type"] != "guide"}

    if "claude-code" in configured_names:
        print(f"  Claude Code  →  /hipcortex remember 'your note'")
        print(f"                  /hipcortex recall 'query'")
    if configured_names & {"cursor", "windsurf", "cline", "roocode", "vscode"}:
        print(f"  Cursor/etc   →  restart IDE · use hipcortex MCP tools")
    if guide_items:
        print(f"\n  {_DIM}Setup guides:{_RESET}")
        for a in guide_items:
            print(f"    {a['name']}: {a['guide']}")
    print(f"\n  Docs: {_CYAN}https://github.com/farmountain/HipCortex{_RESET}\n")

    # Auto-start the server if binary was downloaded and server isn't already running
    if binary_path and binary_path.exists():
        health_url = f"http://localhost:3030/health"
        already_running = False
        try:
            with urllib.request.urlopen(health_url, timeout=1) as r:
                already_running = r.status == 200
        except Exception:
            pass

        if not already_running:
            import subprocess as _sp
            data_dir = str(INSTALL_DIR / "data")
            import pathlib as _pl
            _pl.Path(data_dir).mkdir(parents=True, exist_ok=True)
            env = os.environ.copy()
            env["PORT"] = "3030"
            env["DATA_DIR"] = data_dir
            env["RUST_LOG"] = "warn"
            _sp.Popen(
                [str(binary_path)],
                env=env,
                stdout=_sp.DEVNULL,
                stderr=_sp.DEVNULL,
            )
            # Wait for startup
            import time as _time
            print("Starting HipCortex server...", end=" ", flush=True)
            for _ in range(20):
                _time.sleep(0.5)
                try:
                    with urllib.request.urlopen(health_url, timeout=1) as r:
                        if r.status == 200:
                            print("✓ running on http://localhost:3030")
                            break
                except Exception:
                    pass
            else:
                print("(starting in background)")


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
    # Poll /health until server is ready (max 10 seconds)
    import time
    health_url = f"http://localhost:{port}/health"
    for _ in range(20):
        time.sleep(0.5)
        try:
            with urllib.request.urlopen(health_url, timeout=1) as r:
                if r.status == 200:
                    print(f"✓ HipCortex running on http://localhost:{port}")
                    print(f"  /hipcortex remember 'your note'   (Claude Code)")
                    print(f"  curl {health_url}")
                    print()
                    break
        except Exception:
            pass
    else:
        print("  Server may still be starting... check health manually.")
        print(f"  curl {health_url}")
        print()
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

def cmd_backup(args: argparse.Namespace) -> None:
    """Export all memory records to a JSON backup file."""
    url = args.url or os.environ.get("HIPCORTEX_URL", DEFAULT_URL)
    output = args.output or f"hipcortex-backup-{int(__import__('time').time())}.json"

    try:
        import urllib.request
        export_url = f"{url}/memory/export"
        if args.actor:
            export_url += f"?actor={args.actor}"
        with urllib.request.urlopen(export_url, timeout=30) as r:
            data = r.read().decode("utf-8")
        with open(output, "w", encoding="utf-8") as f:
            f.write(data)
        import json
        records = json.loads(data).get("records", [])
        print(f"✓ Backed up {len(records)} records to {output}")
    except Exception as e:
        print(f"✗ Backup failed: {e}", file=__import__('sys').stderr)
        __import__('sys').exit(1)


def cmd_restore(args: argparse.Namespace) -> None:
    """Restore memory records from a backup file."""
    url = args.url or os.environ.get("HIPCORTEX_URL", DEFAULT_URL)
    if not __import__('os').path.exists(args.file):
        print(f"✗ File not found: {args.file}", file=__import__('sys').stderr)
        __import__('sys').exit(1)

    import json, urllib.request, urllib.error
    with open(args.file, "r", encoding="utf-8") as f:
        backup = json.load(f)
    records = backup.get("records", [])
    if not records:
        print("No records to restore.")
        return

    bulk_payload = json.dumps({"records": [
        {"actor": r["actor"], "action": r["action"], "target": r["target"],
         "record_type": r.get("record_type", "Temporal"),
         "metadata": r.get("metadata", {})}
        for r in records
    ]}).encode("utf-8")

    req = urllib.request.Request(
        f"{url}/memory/bulk",
        data=bulk_payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            result = json.loads(r.read().decode("utf-8"))
        print(f"✓ Restored {result.get('inserted', 0)} records from {args.file}")
        if result.get("failed", 0):
            print(f"  {result['failed']} failed: {result.get('errors', [])[:3]}")
    except Exception as e:
        print(f"✗ Restore failed: {e}", file=__import__('sys').stderr)
        __import__('sys').exit(1)


# ─── Argument parser ──────────────────────────────────────────────────────────

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="hipcortex",
        description="HipCortex memory engine — persistent causal memory for AI agents",
    )
    sub = parser.add_subparsers(dest="command")

    # install
    p_install = sub.add_parser("install", help="Interactive wizard — download binary + configure AI coding assistants")
    p_install.add_argument("--url", help=f"Use an existing server instead of local binary (e.g. {MANAGED_URL})")
    p_install.add_argument("--force", action="store_true", help="Re-download binary even if it exists")
    p_install.add_argument("--yes", "-y", action="store_true", help="Non-interactive: configure all supported agents")

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

    # backup
    p_backup = sub.add_parser("backup", help="Export memory records to JSON file")
    p_backup.add_argument("--url", help="Server URL")
    p_backup.add_argument("--output", "-o", help="Output file path")
    p_backup.add_argument("--actor", help="Filter by actor (optional)")

    # restore
    p_restore = sub.add_parser("restore", help="Restore memory records from backup file")
    p_restore.add_argument("file", help="Backup file path (.json)")
    p_restore.add_argument("--url", help="Server URL")

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
    elif args.command == "backup":
        cmd_backup(args)
    elif args.command == "restore":
        cmd_restore(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
