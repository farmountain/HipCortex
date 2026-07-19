"""HipCortex CLI — install command auto-configures AI coding assistants.

Usage:
    hipcortex install           # download binary + configure Claude Code + Cursor
    hipcortex install --url URL # use existing server instead of local binary
    hipcortex install --mode proactive  # substrate-first harness (MUST get_live_beliefs first etc)
    hipcortex install --mode proactive --no-index  # skip optional codebase index bootstrap
    hipcortex install --index           # force index bootstrap (even in conservative mode)
    hipcortex start             # start the local server (downloads if needed)
    hipcortex status            # check server health
    hipcortex doctor            # post-install verification (health + version)
    hipcortex doctor --probe    # also POST /memory/add + search roundtrip
    hipcortex channels          # print channel honesty matrix (docs/channels.yaml)
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
DEFAULT_URL = "http://127.0.0.1:3030"
MANAGED_URL = "https://hipcortex.fly.dev"

# Daemon paths — re-exported for callers; source of truth is hipcortex.daemon
try:
    from .daemon import DEFAULT_DATA_DIR, LOCK_FILE, PID_FILE
except ImportError:  # pragma: no cover
    from hipcortex.daemon import DEFAULT_DATA_DIR, LOCK_FILE, PID_FILE  # type: ignore


# --- Host installers (re-export for tests / public API stability) ---
from .install_hosts import (  # noqa: F401
    INSTALL_CREATED,
    INSTALL_DRY_RUN,
    INSTALL_REFUSED,
    INSTALL_SKIPPED,
    INSTALL_UNCHANGED,
    INSTALL_UPDATED,
    KNOWN_UNINSTALL_CHANNELS,
    _CLAUDE_REGISTRATION,
    _UNINSTALL_CHANNEL_ALIASES,
    _antigravity_mcp_path,
    _atomic_write_text,
    _build_skill_content,
    _cursor_legacy_global_mcp_path,
    _cursor_mcp_path,
    _desired_grok_toml_block,
    _migrate_legacy_win_cursor_global,
    _desired_mcp_entry,
    _grok_config_path,
    _grok_entry_matches,
    _hermes_entry_matches,
    _hermes_hipcortex_yaml,
    _hermes_merge_yaml,
    _install_antigravity,
    _install_claude_code,
    _install_cursor,
    _install_cursor_prefer_local,
    _install_grok,
    _install_hermes,
    _install_mcp_generic,
    _install_mcp_server,
    _install_openclaw,
    _install_vscode,
    _openclaw_refuse,
    _install_windsurf,
    _openclaw_config_path,
    _proactive_claude_registration,
    _remove_mcp_entry,
    _resolve_uninstall_channels,
    _skill_dir,
    _strip_claude_hipcortex_block,
    _strip_grok_hipcortex_tables,
    _uninstall_antigravity,
    _uninstall_channel,
    _uninstall_claude_code,
    _uninstall_cursor,
    _uninstall_grok,
    _uninstall_hermes,
    _uninstall_mcp_path,
    _uninstall_openclaw,
    _uninstall_vscode,
    _uninstall_windsurf,
    _vscode_settings_path,
    _windsurf_base,
    _write_mcp_servers,
    # colors used by wizard / cmd_* prints
    _BOLD,
    _CLEAR_LINE,
    _CYAN,
    _DIM,
    _GRAY,
    _GREEN,
    _RESET,
)

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



_SPLASH = f"""{_CYAN}{_BOLD}
  ██╗  ██╗██╗██████╗  ██████╗ ██████╗ ██████╗ ████████╗███████╗██╗  ██╗
  ██║  ██║██║██╔══██╗██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██╔════╝╚██╗██╔╝
  ███████║██║██████╔╝██║     ██║   ██║██████╔╝   ██║   █████╗   ╚███╔╝
  ██╔══██║██║██╔═══╝ ██║     ██║   ██║██╔══██╗   ██║   ██╔══╝   ██╔██╗
  ██║  ██║██║██║     ╚██████╗╚██████╔╝██║  ██║   ██║   ███████╗██╔╝ ██╗
  ╚═╝  ╚═╝╚═╝╚═╝      ╚═════╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝  ╚═╝
{_RESET}  Persistent causal memory for AI agents · {_CYAN}hipcortex.fly.dev{_RESET}
"""

# --- Framework scaffold + agent registry (re-export for tests / public API) ---
from .framework_scaffold import (  # noqa: F401, E402
    _FRAMEWORK_TEMPLATE_FILES,
    _build_agent_registry,
    _detect_frameworks,
    _load_framework_template,
    _resolve_framework_code,
    _templates_dir,
    _write_framework_starter,
)


# Core hosts shown when no product dirs are detected on the machine.
_CORE_DEFAULT_HOSTS = frozenset({"claude-code", "cursor", "vscode", "grok-build"})


def _enable_windows_vt() -> bool:
    """Enable VT processing on Windows stdout. True if ANSI clear/redraw is safe."""
    if platform.system() != "Windows":
        return True
    try:
        import ctypes

        kernel32 = ctypes.windll.kernel32  # type: ignore[attr-defined]
        # STD_OUTPUT_HANDLE = -11
        handle = kernel32.GetStdHandle(-11)
        mode = ctypes.c_uint32()
        if not kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            return False
        enable_vt = 0x0004  # ENABLE_VIRTUAL_TERMINAL_PROCESSING
        if not kernel32.SetConsoleMode(handle, mode.value | enable_vt):
            return False
        return True
    except Exception:
        return False


def _detect_host_presence() -> set[str]:
    """Return set of agent ids whose install config dirs / product dirs exist."""
    present: set[str] = set()
    home = Path.home()
    cwd = Path.cwd()

    if (home / ".claude").exists():
        present.add("claude-code")

    if (cwd / ".cursor").exists():
        present.add("cursor")
    else:
        try:
            global_mcp = _cursor_mcp_path(True)
            if global_mcp is not None and global_mcp.parent.exists():
                present.add("cursor")
        except Exception:
            pass

    try:
        if _windsurf_base().exists():
            present.add("windsurf")
    except Exception:
        pass

    try:
        if _vscode_settings_path().parent.exists():
            present.add("vscode")
    except Exception:
        pass

    if (cwd / ".cline").exists():
        present.add("cline")
    if (cwd / ".roo").exists():
        present.add("roocode")
    if (home / ".gemini" / "antigravity").exists():
        present.add("antigravity")
    if (home / ".hermes").exists():
        present.add("hermes")
    if (home / ".openclaw").exists():
        present.add("openclaw")
    if (home / ".grok").exists():
        present.add("grok-build")

    return present


def _filter_agents_for_install(
    agents: list,
    *,
    show_all: bool = False,
    scaffold: bool = False,
) -> list:
    """Slim install list: drop guides; frameworks only if scaffold; presence filter.

    - Drop type==guide always (callers print guide footer separately).
    - Drop type==framework unless scaffold=True.
    - If not show_all: keep native/mcp only if id in presence (empty presence →
      core defaults: claude-code, cursor, vscode, grok-build).
    - Keep type==section only when following non-section rows remain.
    - If show_all: all native/mcp (+ frameworks if scaffold).
    """
    presence = _detect_host_presence()
    if not presence:
        presence = set(_CORE_DEFAULT_HOSTS)

    provisional: list = []
    for a in agents:
        t = a.get("type")
        if t == "guide":
            continue
        if t == "framework":
            if scaffold:
                provisional.append(a)
            continue
        if t == "section":
            provisional.append(a)
            continue
        if t in ("native", "mcp"):
            if show_all or a.get("id") in presence:
                provisional.append(a)
            continue

    # Drop section headers with no following non-section rows before next section
    out: list = []
    for i, a in enumerate(provisional):
        if a.get("type") == "section":
            has_follow = False
            for b in provisional[i + 1 :]:
                if b.get("type") == "section":
                    break
                has_follow = True
                break
            if has_follow:
                out.append(a)
        else:
            out.append(a)
    return out


def _wizard_frame_text(
    agents: list,
    selected: set,
    cursor: int,
    *,
    hint: str = "(Space toggle · Enter confirm · a show all · q quit)",
) -> tuple[str, int]:
    """Build one wizard frame. Returns (text ending with newline, line_count)."""
    import re as _re

    selectable = [i for i, a in enumerate(agents) if a["type"] != "section"]
    cur_real = selectable[cursor] if selectable and 0 <= cursor < len(selectable) else -1

    def _display_name(a: dict) -> str:
        return _re.sub(r"\033\[[^m]*m", "", a["name"])

    lines: list[str] = []
    lines.append(
        f"  {_BOLD}Select what to configure:{_RESET} {_GRAY}{hint}{_RESET}"
    )
    lines.append("")
    for i, agent in enumerate(agents):
        if agent["type"] == "section":
            lines.append(f"  {_DIM}{agent['name']}{_RESET}")
            continue
        is_sel = i in selected
        is_cur = i == cur_real
        bullet = f"{_CYAN}●{_RESET}" if is_sel else f"{_GRAY}○{_RESET}"
        raw_name = _display_name(agent)
        name_str = f"{_BOLD}{agent['name']}{_RESET}" if is_cur else agent["name"]
        desc = f"{_DIM}{agent['desc']}{_RESET}"
        if agent["type"] == "guide":
            tag = f" {_CYAN}[guide]{_RESET}"
        elif agent["type"] == "framework":
            tag = f" {_CYAN}[starter file]{_RESET}"
        else:
            tag = ""
        prefix = "  ›" if is_cur else "   "
        pad = max(1, 20 - len(raw_name))
        lines.append(f"{prefix} {bullet} {name_str}{' ' * pad}{desc}{tag}")
    count = len(selected)
    lines.append("")
    if count:
        lines.append(f"  {_GREEN}{count} selected{_RESET}")
    else:
        lines.append(f"  {_GRAY}none selected{_RESET}")
    text = "\n".join(lines) + "\n"
    return text, len(lines)


def _wizard_clear_previous(frame_lines: int) -> None:
    """Move cursor up frame_lines and erase to end of screen (ANSI VT)."""
    if frame_lines <= 0:
        return
    sys.stdout.write(f"\033[{frame_lines}A\033[J")
    sys.stdout.flush()


def _getch() -> str:
    """Read a single raw keypress.

    Returns: 'up' | 'down' | 'space' | 'enter' | 'quit' | 'all' | ''.
    """
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
        if ch in (b"a", b"A"):
            return "all"
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
            if ch in ("a", "A"):
                return "all"
            return ""
        finally:
            termios.tcsetattr(fd, termios.TCSADRAIN, old)


def _run_wizard(
    agents: list,
    *,
    full_agents: Optional[list] = None,
    scaffold: bool = False,
    show_all: bool = False,
) -> list:
    """Interactive multi-select. Skips section headers. Returns selected agent dicts.

    Uses VT cursor-up + erase redraw (Unix + Windows when VT enabled).
    On Windows when VT enable fails: print list once + number input fallback.
    Key ``a`` toggles show-all when *full_agents* is provided.
    """
    import re as _re

    full = full_agents if full_agents is not None else agents
    show_all_state = show_all
    current: list = list(agents)
    selected_ids: set = set()
    sel_cursor = 0
    frame_lines = 0
    first_draw = True

    use_live = _enable_windows_vt() if platform.system() == "Windows" else True

    def _selectable(lst: list) -> list:
        return [i for i, a in enumerate(lst) if a["type"] != "section"]

    def _selected_indices(lst: list) -> set:
        return {
            i
            for i, a in enumerate(lst)
            if a.get("id") in selected_ids and a["type"] != "section"
        }

    def _refilter() -> None:
        nonlocal current, sel_cursor
        current = _filter_agents_for_install(
            full, show_all=show_all_state, scaffold=scaffold
        )
        sel_cursor = 0

    if not use_live:
        # Non-stacking fallback: optional show-all, then static list + numbers
        if full_agents is not None and not show_all_state:
            try:
                ans = input("  Show all agents? [y/N]: ").strip().lower()
            except EOFError:
                ans = ""
            if ans in ("y", "yes"):
                show_all_state = True
                _refilter()
        selectable = _selectable(current)
        print(
            f"  {_BOLD}Select what to configure:{_RESET} "
            f"{_GRAY}(enter space-separated numbers 1-based; empty=none){_RESET}"
        )
        print()
        for n, idx in enumerate(selectable, 1):
            a = current[idx]
            raw = _re.sub(r"\033\[[^m]*m", "", a["name"])
            print(f"  {n}. {raw}  {_DIM}{a.get('desc', '')}{_RESET}")
        print()
        try:
            raw_in = input("  Numbers: ").strip()
        except EOFError:
            raw_in = ""
        if not raw_in:
            return []
        chosen: list = []
        for part in raw_in.replace(",", " ").split():
            try:
                n = int(part)
            except ValueError:
                continue
            if 1 <= n <= len(selectable):
                chosen.append(current[selectable[n - 1]])
        return chosen

    while True:
        selectable = _selectable(current)
        if selectable:
            sel_cursor = sel_cursor % len(selectable)
        else:
            sel_cursor = 0
        text, n_lines = _wizard_frame_text(
            current, _selected_indices(current), sel_cursor
        )
        if not first_draw:
            _wizard_clear_previous(frame_lines)
        first_draw = False
        sys.stdout.write(text)
        sys.stdout.flush()
        frame_lines = n_lines

        try:
            key = _getch()
        except Exception:
            break

        if key == "quit":
            selected_ids.clear()
            break
        elif key == "up" and selectable:
            sel_cursor = (sel_cursor - 1) % len(selectable)
        elif key == "down" and selectable:
            sel_cursor = (sel_cursor + 1) % len(selectable)
        elif key == "space" and selectable:
            real_i = selectable[sel_cursor]
            aid = current[real_i].get("id")
            if aid in selected_ids:
                selected_ids.discard(aid)
            else:
                selected_ids.add(aid)
        elif key == "all" and full_agents is not None:
            show_all_state = not show_all_state
            _refilter()
        elif key == "enter":
            break

    print()
    # Resolve selected agents from full registry by id (stable across refilter)
    if not selected_ids:
        return []
    by_id = {a.get("id"): a for a in full if a.get("type") != "section"}
    return [by_id[i] for i in sorted(selected_ids) if i in by_id]


# ─── Install: optional codebase index bootstrap ──────────────────────────────

_INDEX_SOURCE_EXTS = {".py", ".ts", ".js", ".rs", ".go", ".java", ".tsx", ".jsx"}
_INDEX_SKIP_DIRS = {
    "node_modules", ".git", "__pycache__", ".venv", "venv", "dist", "build",
    "target", ".tox", ".mypy_cache",
}


def _resolve_install_index_flag(args: argparse.Namespace, mode: str, dry_run: bool) -> bool:
    """Whether install should attempt codebase index bootstrap.

    - dry_run: never
    - --index: always (when not dry_run)
    - --no-index: never
    - default None: True only when mode == proactive
    MagicMock / non-bool attrs treated as None (auto).
    """
    if dry_run:
        return False
    raw = getattr(args, "index", None)
    if raw is True:
        return True
    if raw is False:
        return False
    # None or non-bool (e.g. MagicMock): mode default
    return mode == "proactive"


def _looks_like_codebase(root: Path) -> bool:
    """True if root is a git repo or has shallow source files."""
    try:
        git = root / ".git"
        if git.is_dir() or git.is_file():
            return True
        for p in root.iterdir():
            if p.is_file() and p.suffix in _INDEX_SOURCE_EXTS:
                return True
            if (
                p.is_dir()
                and not p.name.startswith(".")
                and p.name not in _INDEX_SKIP_DIRS
            ):
                try:
                    for child in p.iterdir():
                        if child.is_file() and child.suffix in _INDEX_SOURCE_EXTS:
                            return True
                except OSError:
                    continue
    except OSError:
        return False
    return False


def _server_healthy(url: str, timeout: float = 2.0) -> bool:
    """Best-effort GET {url}/health."""
    base = url.rstrip("/")
    try:
        with urllib.request.urlopen(f"{base}/health", timeout=timeout) as r:
            return r.status == 200
    except Exception:
        return False


def _bootstrap_codebase_index(
    server_url: str,
    path: Optional[Path] = None,
    actor: Optional[str] = None,
) -> bool:
    """Best-effort CodeIndexer on path (default cwd). Never raises. Returns True if ran ok."""
    root = path if path is not None else Path.cwd()
    act = actor or "codebase"
    try:
        if not _looks_like_codebase(root):
            print(
                f"  {_GRAY}–{_RESET} Codebase index        "
                f"{_DIM}skipped (cwd not a git/source tree){_RESET}"
            )
            return False
        if not _server_healthy(server_url):
            print(
                f"  {_GRAY}–{_RESET} Codebase index        "
                f"{_DIM}skipped (server not healthy at {server_url}){_RESET}"
            )
            return False
        try:
            from .client import HipCortexClient
            from .indexer import CodeIndexer
        except ImportError:
            from hipcortex.client import HipCortexClient
            from hipcortex.indexer import CodeIndexer

        client = HipCortexClient(base_url=server_url.rstrip("/"))
        indexer = CodeIndexer(client=client)
        print(f"  Indexing codebase at {root} ...")
        stats = indexer.index(path=str(root), actor=act)
        print(
            f"  {_GREEN}✓{_RESET} Codebase index        "
            f"{_DIM}{stats.get('files', 0)} files, "
            f"{stats.get('nodes', 0)} nodes, "
            f"{stats.get('edges', 0)} edges{_RESET}"
        )
        return True
    except Exception as e:
        print(
            f"  {_GRAY}–{_RESET} Codebase index        "
            f"{_DIM}skipped ({e}){_RESET}"
        )
        return False


# ─── Commands ────────────────────────────────────────────────────────────────

def cmd_install(args: argparse.Namespace) -> None:
    """Interactive wizard: download binary + configure chosen AI coding assistants."""
    print(_SPLASH)

    # Use `is True` so MagicMock auto-attrs in tests don't become truthy.
    dry_run = getattr(args, "dry_run", False) is True
    scaffold = getattr(args, "scaffold", False) is True

    # Non-TTY without --yes: auto-enable --yes (safe full subset refined later in 5C)
    if getattr(args, "yes", False) is not True and not sys.stdin.isatty():
        args.yes = True
        print(
            f"  {_GRAY}Non-TTY stdin: auto-enabling --yes "
            f"(pass --yes explicitly to silence this message){_RESET}\n"
        )

    if dry_run:
        print(f"  {_CYAN}[dry-run]{_RESET} No files will be written; no binary download.\n")

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
        elif dry_run:
            print(
                f"  {_CYAN}[dry-run]{_RESET} would download binary "
                f"for {os_name}/{arch} → {binary_path}"
            )
        else:
            print(f"  Downloading binary for {os_name}/{arch}...")
            _download_binary(_binary_url(os_name, arch), binary_path)
            print(f"  {_GREEN}✓{_RESET} Binary: {binary_path}")

    # ── 2. MCP server script ──────────────────────────────────────────────────
    if dry_run:
        print(f"  {_CYAN}[dry-run]{_RESET} would install MCP server script → ~/.hipcortex-mcp/server.py")
    else:
        _install_mcp_server()

    # ── 3. Agent selection ────────────────────────────────────────────────────
    agents = _build_agent_registry(server_url, sys.executable)

    # Support --mode proactive for claude (surgical: patch claude fn only; other fns unchanged)
    mode = getattr(args, "mode", "conservative")
    actor = getattr(args, "actor", None)
    for a in agents:
        if a.get("id") == "claude-code":
            a["fn"] = lambda act=actor: (
                _install_claude_code(server_url, mode, actor=act),
                "~/.claude/skills/hipcortex/",
            )
            break

    yes_all = getattr(args, "yes", False) is True
    if yes_all:
        # Non-interactive: all native/mcp; frameworks only with --scaffold; never guides
        chosen = [
            a
            for a in agents
            if a["type"] in ("native", "mcp")
            or (scaffold and a["type"] == "framework")
        ]
        print(f"  --yes: configuring all {len(chosen)} supported agents\n")
    else:
        display = _filter_agents_for_install(
            agents, show_all=False, scaffold=scaffold
        )
        if not any(a.get("type") in ("native", "mcp", "framework") for a in display):
            # Should not happen with core defaults; fall back to show-all hosts
            display = _filter_agents_for_install(
                agents, show_all=True, scaffold=scaffold
            )
        n_hosts = sum(
            1 for a in display if a.get("type") in ("native", "mcp", "framework")
        )
        # Live VT: key `a`. Win no-VT: pre-prompt inside _run_wizard.
        print(
            f"  Showing {n_hosts} detected hosts "
            f"(show all: press a in live wizard, or answer y on fallback prompt)\n"
        )
        print(f"  {_BOLD}Which AI coding assistants do you use?{_RESET}\n")
        chosen = _run_wizard(
            display, full_agents=agents, scaffold=scaffold, show_all=False
        )

    if not chosen:
        print(f"  {_GRAY}Nothing selected. Run 'hipcortex install' again anytime.{_RESET}\n")
        # Still show docs guides when user aborts selection
        guide_items = [a for a in agents if a.get("type") == "guide"]
        if guide_items:
            print(f"\n  {_DIM}Setup guides:{_RESET}")
            import re as _re_guides
            for a in guide_items:
                raw = _re_guides.sub(r"\033\[[^m]*m", "", a.get("name", ""))
                print(f"    {raw}: {a.get('guide', '')}")
            print()
        return

    # ── 4. Configure chosen agents ────────────────────────────────────────────
    print(f"\n  {_BOLD}{'Would configure' if dry_run else 'Configuring'}:{_RESET}\n")

    import re as _re
    framework_files = []
    status_counts: dict[str, int] = {
        INSTALL_CREATED: 0,
        INSTALL_UPDATED: 0,
        INSTALL_UNCHANGED: 0,
        INSTALL_SKIPPED: 0,
        INSTALL_REFUSED: 0,
        INSTALL_DRY_RUN: 0,
        "error": 0,
    }

    def _bump(status: str) -> None:
        status_counts[status] = status_counts.get(status, 0) + 1

    for agent in chosen:
        if agent["type"] == "section":
            continue
        name = _re.sub(r'\033\[[^m]*m', '', agent["name"])  # strip ANSI for display width
        if agent["type"] == "guide":
            # Guides are not selectable; skip if present in chosen
            continue
        if agent["type"] == "framework":
            fname = agent.get("file", "starter.py")
            if not scaffold:
                print(
                    f"  {_GRAY}–{_RESET} {name:<22} "
                    f"{_DIM}package API only (pass --scaffold to write {fname}){_RESET}"
                )
                _bump(INSTALL_SKIPPED)
                continue
            if dry_run:
                dest = str(Path.cwd() / fname)
                print(f"  {_CYAN}[dry-run]{_RESET} would write {name:<14} → {dest}")
                framework_files.append((name, dest))
                _bump(INSTALL_DRY_RUN)
                continue
            try:
                ok, dest = _write_framework_starter(fname, agent["code"])
                if ok:
                    print(
                        f"  {_GREEN}✓{_RESET} {name:<22} "
                        f"{_CYAN}{INSTALL_CREATED:<10}{_RESET} {_DIM}{dest}{_RESET}"
                    )
                    framework_files.append((name, dest))
                    _bump(INSTALL_CREATED)
                else:
                    print(f"  {_GRAY}–{_RESET} {name:<22} {_DIM}skipped{_RESET}")
                    _bump(INSTALL_SKIPPED)
            except Exception as e:
                print(f"  ✗ {name:<22} error: {e}")
                _bump("error")
            continue
        # native / mcp agents — never call installer fn in dry-run
        if dry_run:
            print(f"  {_CYAN}[dry-run]{_RESET} would install {name}")
            _bump(INSTALL_DRY_RUN)
            continue
        try:
            status, detail = agent["fn"]()
            # Back-compat if a custom fn still returns bool
            if isinstance(status, bool):
                status = INSTALL_CREATED if status else INSTALL_SKIPPED
            if status == INSTALL_SKIPPED:
                print(
                    f"  {_GRAY}–{_RESET} {name:<22} "
                    f"{_DIM}{status:<10} not found (install first){_RESET}"
                )
            elif status == INSTALL_REFUSED:
                print(
                    f"  {_GRAY}–{_RESET} {name:<22} "
                    f"{_DIM}{status:<10} corrupt config (primary not overwritten){_RESET}"
                )
            elif status == INSTALL_UNCHANGED:
                print(
                    f"  {_GREEN}✓{_RESET} {name:<22} "
                    f"{_DIM}{status:<10}{detail}{_RESET}"
                )
            else:
                print(
                    f"  {_GREEN}✓{_RESET} {name:<22} "
                    f"{_CYAN}{status:<10}{_RESET} {_DIM}{detail}{_RESET}"
                )
            _bump(status)
        except Exception as e:
            print(f"  ✗ {name:<22} error: {e}")
            _bump("error")

    # ── 4b. Project config (.hipcortex/config.toml) ───────────────────────────
    channel_ids = [
        a["id"]
        for a in chosen
        if a.get("type") not in ("guide", "section") and a.get("id")
        and (a.get("type") != "framework" or scaffold)
    ]
    if dry_run:
        print(
            f"  {_CYAN}[dry-run]{_RESET} would write project config "
            f"→ {Path.cwd() / '.hipcortex' / 'config.toml'} "
            f"(channels={channel_ids})"
        )
    else:
        try:
            try:
                from .config import ensure_project_config
            except ImportError:
                from hipcortex.config import ensure_project_config

            cfg_path = ensure_project_config(
                Path.cwd(),
                url=server_url,
                actor=actor,
                mode=mode or "conservative",
                channels=channel_ids,
            )
            print(f"  {_GREEN}✓{_RESET} Project config        {_DIM}{cfg_path}{_RESET}")
        except Exception as e:
            print(f"  {_GRAY}–{_RESET} Project config        {_DIM}skipped ({e}){_RESET}")

    # ── 5. Start server ───────────────────────────────────────────────────────
    if dry_run:
        if binary_path:
            print(f"  {_CYAN}[dry-run]{_RESET} would start server if binary present and not running")
    elif binary_path and binary_path.exists():
        health_url = "http://localhost:3030/health"
        already_running = False
        try:
            with urllib.request.urlopen(health_url, timeout=1) as r:
                already_running = r.status == 200
        except Exception:
            pass

        if not already_running:
            import subprocess as _sp
            try:
                from .config import load_settings as _ls
                from .daemon import resolve_start_data_dir as _rsdd
            except ImportError:  # pragma: no cover
                from hipcortex.config import load_settings as _ls  # type: ignore
                from hipcortex.daemon import resolve_start_data_dir as _rsdd  # type: ignore

            data_dir = _rsdd(
                settings_data_dir=_ls().data_dir,
                default=DEFAULT_DATA_DIR,
            )
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

    # ── 5a. Optional codebase index bootstrap ─────────────────────────────────
    # proactive default: index unless --no-index; conservative: only if --index
    if dry_run and _resolve_install_index_flag(
        args, mode or "conservative", dry_run=False
    ):
        # dry_run forces False in resolver; re-check mode default for message only
        print(
            f"  {_CYAN}[dry-run]{_RESET} would bootstrap codebase index "
            f"at {Path.cwd()} (server healthy + source tree)"
        )
    elif _resolve_install_index_flag(args, mode or "conservative", dry_run):
        _bootstrap_codebase_index(
            server_url,
            path=Path.cwd(),
            actor=actor or "codebase",
        )

    # ── 5b. Install summary counts ────────────────────────────────────────────
    summary_parts = [
        f"{n} {label}"
        for label, n in status_counts.items()
        if n > 0
    ]
    if summary_parts:
        print(f"\n  {_BOLD}Install summary:{_RESET} " + ", ".join(summary_parts))

    # ── 6. Usage hints ────────────────────────────────────────────────────────
    if dry_run:
        print(f"\n  {_BOLD}Dry-run complete — no changes made.{_RESET}\n")
    else:
        print(f"\n  {_BOLD}Ready!{_RESET}\n")
    configured_names = {
        a["id"]
        for a in chosen
        if a["type"] not in ("guide", "section")
        and (a["type"] != "framework" or scaffold)
    }

    if "claude-code" in configured_names:
        print(f"  Claude Code  →  /hipcortex remember 'your note'")
        print(f"                  /hipcortex recall 'query'")
    if configured_names & {"cursor", "windsurf", "cline", "roocode", "vscode"}:
        print(f"  Cursor/etc   →  restart IDE · use hipcortex MCP tools")
    if framework_files:
        label = "Starter files planned:" if dry_run else "Starter files written:"
        print(f"\n  {_BOLD}{label}{_RESET}")
        for fname, fpath in framework_files:
            print(f"  {_GREEN}✓{_RESET} {fname:<20} {_DIM}{fpath}{_RESET}")
        if not dry_run:
            print(f"  {_DIM}Edit these files and import into your project.{_RESET}")
    elif any(a.get("type") == "framework" for a in chosen) and not scaffold:
        print(
            f"\n  {_DIM}Framework starters skipped (default). "
            f"Re-run with --scaffold to write hipcortex_*.py to cwd.{_RESET}"
        )
    # Guides never selectable — always print compact footer from full registry
    guide_items = [a for a in agents if a.get("type") == "guide"]
    if guide_items:
        print(f"\n  {_DIM}Setup guides:{_RESET}")
        for a in guide_items:
            raw = _re.sub(r'\033\[[^m]*m', '', a["name"])
            guide_url = a.get("guide", "")
            print(f"    {raw}: {guide_url}")
    print(f"\n  Docs: {_CYAN}https://github.com/farmountain/HipCortex{_RESET}\n")

    # Auto-start the server if binary was downloaded and server isn't already running
    if dry_run:
        return
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
            import pathlib as _pl
            try:
                from .config import load_settings as _ls
                from .daemon import resolve_start_data_dir as _rsdd
            except ImportError:  # pragma: no cover
                from hipcortex.config import load_settings as _ls  # type: ignore
                from hipcortex.daemon import resolve_start_data_dir as _rsdd  # type: ignore

            data_dir = _rsdd(
                settings_data_dir=_ls().data_dir,
                default=DEFAULT_DATA_DIR,
            )
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


def _load_daemon():
    try:
        from . import daemon as d
    except ImportError:  # pragma: no cover
        import hipcortex.daemon as d  # type: ignore
    return d


def cmd_start(args: argparse.Namespace) -> None:
    """Start the local HipCortex server (shared daemon protocol)."""
    d = _load_daemon()
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

    port = args.port or int(os.environ.get("PORT", str(d.DEFAULT_PORT)))
    try:
        from .config import load_settings as _load_settings
    except ImportError:  # pragma: no cover
        from hipcortex.config import load_settings as _load_settings  # type: ignore

    _settings = _load_settings()
    data_dir = d.resolve_start_data_dir(
        getattr(args, "data_dir", None),
        settings_data_dir=_settings.data_dir,
        default=d.DEFAULT_DATA_DIR,
    )

    print(f"Starting HipCortex on http://127.0.0.1:{port}")
    print(f"Data: {data_dir}")

    try:
        result = d.start_server(binary, port=port, data_dir=data_dir)
    except d.AlreadyRunning as e:
        health = d.health_check(f"http://127.0.0.1:{port}")
        print(f"✓ HipCortex already running on http://127.0.0.1:{port}")
        if health:
            print(f"  Version: {health.get('version', 'unknown')}")
        print(f"  Run 'hipcortex stop' to stop it first.")
        print(f"  ({e})")
        sys.exit(1)
    except d.PortInUse:
        print(f"✗ Port {port} already in use by another application.")
        print("  Use --port to pick a different port.")
        sys.exit(1)
    except d.LockError as e:
        print(f"✗ {e}")
        print("  Another start may be in progress; try 'hipcortex status' or 'hipcortex stop'.")
        sys.exit(1)
    except d.DaemonError as e:
        print(f"✗ Failed to start: {e}", file=sys.stderr)
        sys.exit(1)
    except FileNotFoundError as e:
        print(f"✗ {e}")
        print("Run: hipcortex install")
        sys.exit(1)

    print(f"PID: {result['pid']}")
    if result.get("ready") and result.get("health"):
        ver = result["health"].get("version", "?")
        print(f"✓ HipCortex v{ver} running on {result['url']}")
        print("  /hipcortex remember 'your note'   (Claude Code)")
        print(f"  curl {result['url']}/health")
        print("  hipcortex stop   (to shut down)")
        print()
    else:
        print("  Server may still be starting... check health manually.")


def cmd_stop(args: argparse.Namespace) -> None:
    """Stop the local HipCortex server (PID file, then port fallback)."""
    d = _load_daemon()
    port = args.port or int(os.environ.get("PORT", str(d.DEFAULT_PORT)))
    result = d.stop_server(port=port)

    if result.get("stopped"):
        pid = result.get("pid")
        method = result.get("method")
        if method == "pid" and pid is not None:
            print(f"✓ Sent stop signal to HipCortex (PID {pid})")
            print("  Process exited cleanly.")
        elif pid is not None:
            print(f"✓ Stopped HipCortex on port {port} (PID {pid})")
        else:
            print(f"✓ Stopped HipCortex on port {port}")
        return

    print(f"No HipCortex process found on port {port}.")


def cmd_status(args: argparse.Namespace) -> None:
    """Check server health via shared daemon.health_check."""
    d = _load_daemon()
    url = (args.url or os.environ.get("HIPCORTEX_URL", DEFAULT_URL)).rstrip("/")
    data = d.health_check(url, timeout=5.0)
    if data is not None:
        # compact one-line summary
        if "raw" in data and len(data) <= 2:
            body = data["raw"]
        else:
            body = json.dumps(data, separators=(",", ":"))
        print(f"✓ HipCortex running at {url} ({body})")
        pid = d.read_pid()
        if pid is not None:
            alive = d.is_pid_alive(pid)
            print(f"  PID file: {pid} ({'alive' if alive else 'stale'})")
        if d.LOCK_FILE.exists():
            print(f"  Lock: {d.LOCK_FILE}")
        return
    print(f"✗ Not reachable at {url}")
    print("  Run: hipcortex start")


def cmd_restart(args: argparse.Namespace) -> None:
    """Stop then start local server."""
    d = _load_daemon()
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

    port = args.port or int(os.environ.get("PORT", str(d.DEFAULT_PORT)))
    try:
        from .config import load_settings as _load_settings
    except ImportError:  # pragma: no cover
        from hipcortex.config import load_settings as _load_settings  # type: ignore

    _settings = _load_settings()
    data_dir = d.resolve_start_data_dir(
        getattr(args, "data_dir", None),
        settings_data_dir=_settings.data_dir,
        default=d.DEFAULT_DATA_DIR,
    )
    print(f"Restarting HipCortex on http://127.0.0.1:{port} ...")
    try:
        result = d.restart_server(binary, port=port, data_dir=data_dir)
    except d.DaemonError as e:
        print(f"✗ Restart failed: {e}", file=sys.stderr)
        sys.exit(1)
    except FileNotFoundError as e:
        print(f"✗ {e}")
        sys.exit(1)

    print(f"PID: {result['pid']}")
    if result.get("ready") and result.get("health"):
        ver = result["health"].get("version", "?")
        print(f"✓ HipCortex v{ver} running on {result['url']}")
    else:
        print("  Server may still be starting... check health manually.")


def cmd_doctor(args: argparse.Namespace) -> None:
    """Post-install verification: GET /health, version, optional add/search probe."""
    try:
        from .doctor import doctor_exit_code, format_report, run_doctor
    except ImportError:
        from hipcortex.doctor import doctor_exit_code, format_report, run_doctor

    report = run_doctor(url=getattr(args, "url", None), probe=bool(getattr(args, "probe", False)))
    print(format_report(report))
    sys.exit(doctor_exit_code(report))


def cmd_uninstall(args: argparse.Namespace) -> None:
    """Remove HipCortex channel configs; optional --purge for binary + MCP script."""
    channels = _resolve_uninstall_channels(args)
    print("Uninstalling HipCortex...")
    if not channels:
        print(f"  {_GRAY}No channels selected.{_RESET}")
    else:
        print(f"  Channels: {', '.join(channels)}")

    labels = {
        "claude-code": "Claude Code",
        "cursor": "Cursor",
        "windsurf": "Windsurf",
        "vscode": "VS Code",
        "cline": "Cline",
        "roocode": "RooCode",
        "antigravity": "Antigravity",
        "hermes": "Hermes",
        "openclaw": "OpenClaw",
        "grok": "Grok Build",
    }
    removed_n = 0
    for ch in channels:
        label = labels.get(ch, ch)
        try:
            did = _uninstall_channel(ch)
        except Exception as e:
            print(f"  ✗ {label}: {e}")
            continue
        if did:
            print(f"  {_GREEN}✓{_RESET} Removed from {label}")
            removed_n += 1
        else:
            print(f"  {_GRAY}–{_RESET} {label}: nothing to remove")

    print(f"  Uninstall summary: {removed_n} channel(s) cleaned")

    if getattr(args, "purge", False) is True:
        if INSTALL_DIR.exists():
            shutil.rmtree(INSTALL_DIR)
            print(f"  {_GREEN}✓{_RESET} Deleted {INSTALL_DIR}")
        mcp_dir = Path.home() / ".hipcortex-mcp"
        if mcp_dir.exists():
            shutil.rmtree(mcp_dir)
            print(f"  {_GREEN}✓{_RESET} Deleted {mcp_dir}")

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


def _channels_yaml_candidates() -> list[Path]:
    """Resolve docs/channels.yaml from repo checkout or CWD."""
    here = Path(__file__).resolve()
    # sdk/python/hipcortex/cli.py → repo root is parents[3]
    roots = []
    if len(here.parents) > 3:
        roots.append(here.parents[3])
    roots.append(Path.cwd())
    return [r / "docs" / "channels.yaml" for r in roots]


def _parse_channels_yaml(text: str) -> list[dict]:
    """Minimal YAML list parser for channels.yaml (no PyYAML dep)."""
    channels: list[dict] = []
    current: Optional[dict] = None
    in_channels = False
    for raw in text.splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped == "channels:" or stripped.startswith("channels:"):
            in_channels = True
            continue
        if not in_channels:
            continue
        if stripped.startswith("- id:"):
            if current:
                channels.append(current)
            val = stripped.split(":", 1)[1].strip().strip('"').strip("'")
            current = {"id": val}
            continue
        if current is not None and ":" in stripped and not stripped.startswith("-"):
            key, val = stripped.split(":", 1)
            current[key.strip()] = val.strip().strip('"').strip("'")
    if current:
        channels.append(current)
    return channels


def _fallback_channels() -> list[dict]:
    """Hardcoded subset when channels.yaml is not on disk (PyPI install)."""
    return [
        {"id": "claude-code", "name": "Claude Code", "status": "native", "install": "hipcortex install"},
        {"id": "cursor", "name": "Cursor", "status": "mcp", "install": "hipcortex install"},
        {"id": "windsurf", "name": "Windsurf", "status": "mcp", "install": "hipcortex install"},
        {"id": "vscode-extension", "name": "VS Code VSIX", "status": "native", "install": "hipcortex-memory-0.5.5.vsix"},
        {"id": "vscode-mcp", "name": "VS Code MCP", "status": "mcp", "install": "hipcortex install"},
        {"id": "cline", "name": "Cline", "status": "mcp", "install": "hipcortex install"},
        {"id": "roocode", "name": "RooCode", "status": "mcp", "install": "hipcortex install"},
        {"id": "langchain", "name": "LangChain", "status": "framework", "install": "package + scaffold"},
        {"id": "antigravity", "name": "Antigravity IDE", "status": "mcp", "install": "hipcortex install"},
        {"id": "grok-code", "name": "Grok Code", "status": "mcp", "install": "hipcortex install"},
        {"id": "grok-build", "name": "Grok Build", "status": "mcp", "install": "hipcortex install"},
        {"id": "hermes", "name": "Hermes", "status": "mcp", "install": "hipcortex install"},
        {"id": "openclaw", "name": "OpenClaw", "status": "mcp", "install": "hipcortex install"},
        {"id": "continue", "name": "Continue", "status": "guide", "install": "docs only"},
        {"id": "copilot", "name": "GitHub Copilot", "status": "guide", "install": "docs only"},
    ]


def cmd_channels(args: argparse.Namespace) -> None:
    """Print channel honesty matrix from docs/channels.yaml (or fallback)."""
    source = "embedded fallback"
    channels: list[dict] = []
    for path in _channels_yaml_candidates():
        if path.is_file():
            try:
                channels = _parse_channels_yaml(path.read_text(encoding="utf-8"))
                source = str(path)
                break
            except OSError:
                continue
    if not channels:
        channels = _fallback_channels()

    status_filter = (args.status or "").strip().lower() or None
    if status_filter:
        channels = [c for c in channels if c.get("status", "").lower() == status_filter]

    print(f"HipCortex channels  (source: {source})")
    print("status: native | mcp | framework | guide | claimed | none")
    print(f"{'ID':<18} {'STATUS':<12} {'NAME':<28} INSTALL")
    print("-" * 90)
    for c in channels:
        print(
            f"{c.get('id', '?'):<18} "
            f"{c.get('status', '?'):<12} "
            f"{c.get('name', c.get('id', '?')):<28} "
            f"{c.get('install', '')}"
        )
    print()
    print("Full table: docs/channels.md  ·  registry: docs/channels.yaml")
    if any(c.get("status") == "claimed" for c in channels):
        print("Note: 'claimed' = marketing/docs example only — not first-class install.")


def cmd_index(args: argparse.Namespace) -> None:
    """Index a codebase into HipCortex symbolic knowledge graph."""
    url = args.url or os.environ.get("HIPCORTEX_URL", DEFAULT_URL)
    path = args.path or "."
    actor = args.actor or "codebase"

    # Check server is reachable
    try:
        with urllib.request.urlopen(f"{url}/health", timeout=5) as r:
            if r.status != 200:
                print(f"✗ Server not reachable at {url}")
                sys.exit(1)
    except Exception:
        print(f"✗ Server not reachable at {url}")
        print(f"  Start it with: hipcortex start")
        sys.exit(1)

    try:
        from .client import HipCortexClient
        from .indexer import CodeIndexer
    except ImportError:
        from hipcortex.client import HipCortexClient
        from hipcortex.indexer import CodeIndexer

    client = HipCortexClient(base_url=url)
    indexer = CodeIndexer(client=client)

    extensions = None
    if args.extensions:
        extensions = [e if e.startswith(".") else f".{e}" for e in args.extensions.split(",")]

    print(f"Indexing {path} into HipCortex symbolic graph...")
    print(f"  Server: {url}")
    print(f"  Actor:  {actor}")
    if extensions:
        print(f"  File types: {extensions}")
    print()

    stats = indexer.index(path=path, actor=actor, extensions=extensions)

    print(f"  ✓ {stats['files']} files processed")
    print(f"  ✓ {stats['nodes']} symbol nodes created")
    print(f"  ✓ {stats['edges']} relationships created")
    print()
    print(f"Query code graph: GET {url}/graph")
    print(f"Search symbols:   GET {url}/graph/search?q=<name>")
    print(f"In Claude Code:   /hipcortex recall validate_token")


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
    p_install.add_argument(
        "--scaffold",
        action="store_true",
        help="Write framework starter files to cwd (hipcortex_langchain.py, etc.). Off by default.",
    )
    p_install.add_argument(
        "--dry-run",
        action="store_true",
        help="Print planned actions only — no binary download, config, skill, MCP, or scaffold writes",
    )
    p_install.add_argument("--mode", choices=["conservative", "proactive"], default="conservative", help="SKILL policy: conservative (explicit) or proactive (substrate-first harness; MUST search/get_live_beliefs first)")
    p_install.add_argument("--actor", help="Configure default actor for this client install")
    p_install.add_argument(
        "--index",
        action=argparse.BooleanOptionalAction,
        default=None,
        help=(
            "Bootstrap codebase index into HipCortex after install "
            "(default: on for --mode proactive; off for conservative). "
            "Use --no-index to skip; --index to force on conservative."
        ),
    )

    # start
    p_start = sub.add_parser("start", help="Start the local HipCortex server")
    p_start.add_argument("--port", type=int, help="Port (default: 3030)")
    p_start.add_argument(
        "--data-dir",
        help=f"Data directory (default: {DEFAULT_DATA_DIR})",
    )

    # status
    p_status = sub.add_parser("status", help="Check server health")
    p_status.add_argument("--url", help="Server URL to check")

    # restart
    p_restart = sub.add_parser("restart", help="Restart the local HipCortex server")
    p_restart.add_argument("--port", type=int, help="Port (default: 3030)")
    p_restart.add_argument(
        "--data-dir",
        help=f"Data directory (default: {DEFAULT_DATA_DIR})",
    )

    # doctor (post-install verification)
    p_doctor = sub.add_parser(
        "doctor",
        help="Post-install verification (GET /health, version; optional --probe)",
    )
    p_doctor.add_argument(
        "--url",
        help=(
            f"Server URL (default: HIPCORTEX_URL > .hipcortex/config.toml "
            f"> user config > {DEFAULT_URL})"
        ),
    )
    p_doctor.add_argument(
        "--probe",
        action="store_true",
        help="Also POST /memory/add + /memory/search roundtrip (online only)",
    )

    # stop
    p_stop = sub.add_parser("stop", help="Stop the local HipCortex server")
    p_stop.add_argument("--port", type=int, help="Port (default: 3030)")

    # uninstall
    p_uninstall = sub.add_parser(
        "uninstall",
        help="Remove HipCortex channel configuration (skill / MCP entries)",
    )
    p_uninstall.add_argument(
        "--channel",
        action="append",
        dest="channel",
        metavar="CHANNEL",
        help=(
            "Channel to remove (repeatable): "
            + ", ".join(KNOWN_UNINSTALL_CHANNELS)
        ),
    )
    p_uninstall.add_argument(
        "--all",
        action="store_true",
        dest="all",
        help="Remove all known channels (default when --channel omitted)",
    )
    p_uninstall.add_argument(
        "--purge",
        action="store_true",
        help="Also delete downloaded binary (~/.hipcortex) and MCP script (~/.hipcortex-mcp)",
    )

    # backup
    p_backup = sub.add_parser("backup", help="Export memory records to JSON file")
    p_backup.add_argument("--url", help="Server URL")
    p_backup.add_argument("--output", "-o", help="Output file path")
    p_backup.add_argument("--actor", help="Filter by actor (optional)")

    # restore
    p_restore = sub.add_parser("restore", help="Restore memory records from backup file")
    p_restore.add_argument("file", help="Backup file path (.json)")
    p_restore.add_argument("--url", help="Server URL")

    # index
    p_index = sub.add_parser("index", help="Index a codebase into the HipCortex knowledge graph")
    p_index.add_argument("path", nargs="?", default=".", help="Directory or file to index (default: current dir)")
    p_index.add_argument("--url", help="Server URL")
    p_index.add_argument("--actor", help="Actor label for this codebase (default: codebase)")
    p_index.add_argument("--extensions", help="Comma-separated file extensions (default: .py,.ts,.js)")

    # channels (Phase 0 honesty matrix)
    p_channels = sub.add_parser(
        "channels",
        help="Print channel honesty matrix (native/mcp/framework/guide/claimed)",
    )
    p_channels.add_argument(
        "--status",
        help="Filter by status (native|mcp|framework|guide|claimed|none)",
    )

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
    elif args.command == "restart":
        cmd_restart(args)
    elif args.command == "doctor":
        cmd_doctor(args)
    elif args.command == "stop":
        cmd_stop(args)
    elif args.command == "uninstall":
        cmd_uninstall(args)
    elif args.command == "backup":
        cmd_backup(args)
    elif args.command == "restore":
        cmd_restore(args)
    elif args.command == "index":
        cmd_index(args)
    elif args.command == "channels":
        cmd_channels(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
