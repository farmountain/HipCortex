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

# Claude Code registration line appended to ~/.claude/CLAUDE.md
_CLAUDE_REGISTRATION = """
# hipcortex
- **hipcortex** (`~/.claude/skills/hipcortex/SKILL.md`) - Persistent memory for AI agents. Store decisions, recall context, GDPR forget. Trigger: `/hipcortex`
When the user types `/hipcortex`, invoke the Skill tool with `skill: "hipcortex"` before doing anything else.
"""

# Install result statuses (idempotent installers)
INSTALL_CREATED = "created"
INSTALL_UPDATED = "updated"
INSTALL_UNCHANGED = "unchanged"
INSTALL_SKIPPED = "skipped"
INSTALL_DRY_RUN = "dry-run"

# Uninstallable MCP / native channels
KNOWN_UNINSTALL_CHANNELS = (
    "claude-code",
    "cursor",
    "windsurf",
    "vscode",
    "cline",
    "roocode",
    "antigravity",
    "hermes",
    "openclaw",
    "grok",
)

# CLI aliases → canonical uninstall channel id
_UNINSTALL_CHANNEL_ALIASES = {
    "grok-build": "grok",
    "grok-code": "grok",
}

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


def _desired_mcp_entry(server_url: str) -> dict:
    """Canonical hipcortex MCP server entry for merge/compare."""
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    return {
        "command": sys.executable,
        "args": [mcp_server_py],
        "env": {"HIPCORTEX_URL": server_url},
    }


def _atomic_write_text(path: Path, content: str) -> None:
    """Write text via temp file + os.replace (same-dir atomic swap)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.parent / f".{path.name}.{os.getpid()}.tmp"
    try:
        tmp.write_text(content, encoding="utf-8")
        os.replace(tmp, path)
    except Exception:
        try:
            if tmp.exists():
                tmp.unlink()
        except OSError:
            pass
        raise


def _write_mcp_servers(mcp_path: Path, server_url: str) -> str:
    """Merge hipcortex into mcpServers JSON.

    Returns created|updated|unchanged|skipped.
    On corrupt JSON or non-dict root/mcpServers: never overwrite primary;
    write sidecar ``{stem}.hipcortex.mcp.json`` and return INSTALL_SKIPPED.
    """
    entry = _desired_mcp_entry(server_url)

    def _skip_corrupt(reason: str) -> str:
        sidecar = mcp_path.parent / f"{mcp_path.stem}.hipcortex.mcp.json"
        mcp_path.parent.mkdir(parents=True, exist_ok=True)
        sidecar.write_text(
            json.dumps({"mcpServers": {"hipcortex": entry}}, indent=2) + "\n",
            encoding="utf-8",
        )
        print(
            f"  {_GRAY}MCP config {mcp_path.name} is corrupt ({reason}); "
            f"original not overwritten. Wrote sidecar {sidecar.name}.{_RESET}"
        )
        return INSTALL_SKIPPED

    existing: dict
    if mcp_path.exists():
        raw = mcp_path.read_text(encoding="utf-8")
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            return _skip_corrupt("invalid JSON")
        if not isinstance(parsed, dict):
            return _skip_corrupt("root is not an object")
        if "mcpServers" in parsed and not isinstance(parsed["mcpServers"], dict):
            return _skip_corrupt("mcpServers is not an object")
        existing = parsed
    else:
        existing = {}

    mcp_servers = existing.setdefault("mcpServers", {})
    prev = mcp_servers.get("hipcortex")
    if prev == entry:
        return INSTALL_UNCHANGED

    had_entry = prev is not None
    mcp_servers["hipcortex"] = entry
    _atomic_write_text(mcp_path, json.dumps(existing, indent=2))
    return INSTALL_UPDATED if had_entry else INSTALL_CREATED


def _remove_mcp_entry(mcp_path: Path) -> bool:
    """Remove hipcortex key from mcpServers. True if something removed."""
    if not mcp_path.exists():
        return False
    try:
        data = json.loads(mcp_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return False
    servers = data.get("mcpServers")
    if not isinstance(servers, dict) or "hipcortex" not in servers:
        return False
    del servers["hipcortex"]
    data["mcpServers"] = servers
    mcp_path.write_text(json.dumps(data, indent=2), encoding="utf-8")
    return True


def _build_skill_content(server_url: str, mode: str = "conservative", actor: str = None) -> str:
    """Render SKILL.md body for the given server/mode/actor."""
    skill_md_src = Path(__file__).parent / "install" / "SKILL.md"
    content = skill_md_src.read_text(encoding="utf-8")
    content = content.replace("http://localhost:3030", server_url)

    if actor:
        content = content.replace(
            "Use the current git repository name as the actor (run `git rev-parse --show-toplevel | xargs basename` to get it). Fall back to \"default\" if not in a git repo.",
            f"Use \"{actor}\" as the actor.",
        )

    if mode == "proactive":
        if "MUST: Before any question" not in content:
            content = content.replace(
                "Invoke HipCortex when the user asks you to:",
                "You are a memory-centric agent. MUST search/get_live_beliefs first (substrate for state/hyp/pred); LLM only final/creative hyp.",
            )
    return content


def _install_claude_code(server_url: str, mode: str = "conservative", actor: str = None) -> str:
    """Write SKILL.md + append to CLAUDE.md. Returns install status string.
    Supports --mode proactive (uses substrate-first template + harness registration).
    """
    claude_dir = Path.home() / ".claude"
    if not claude_dir.exists():
        return INSTALL_SKIPPED  # Claude Code not installed

    skill_dir = _skill_dir()
    skill_file = skill_dir / "SKILL.md"
    content = _build_skill_content(server_url, mode=mode, actor=actor)

    skill_existed = skill_file.exists()
    skill_same = skill_existed and skill_file.read_text(encoding="utf-8") == content

    if not skill_same:
        skill_dir.mkdir(parents=True, exist_ok=True)
        skill_file.write_text(content, encoding="utf-8")

    # Append registration to CLAUDE.md if not already present
    claude_md = claude_dir / "CLAUDE.md"
    existing = claude_md.read_text(encoding="utf-8") if claude_md.exists() else ""
    reg_added = False
    if "hipcortex" not in existing:
        reg = _CLAUDE_REGISTRATION
        if mode == "proactive":
            reg = reg.replace(
                "Persistent memory for AI agents",
                "Proactive substrate-first memory (Claude Agent Harness) for AI agents. MUST search/get_live_beliefs first; use substrate for state/hyp/pred; LLM only final or creative hyp",
            )
        with claude_md.open("a", encoding="utf-8") as f:
            f.write(reg)
        reg_added = True

    if not skill_existed:
        status = INSTALL_CREATED
    elif skill_same:
        status = INSTALL_UNCHANGED
    else:
        status = INSTALL_UPDATED
    if reg_added and status == INSTALL_UNCHANGED:
        status = INSTALL_UPDATED
    return status


def _proactive_claude_registration() -> str:
    """Registration block used when install mode is proactive."""
    return _CLAUDE_REGISTRATION.replace(
        "Persistent memory for AI agents",
        "Proactive substrate-first memory (Claude Agent Harness) for AI agents. "
        "MUST search/get_live_beliefs first; use substrate for state/hyp/pred; "
        "LLM only final or creative hyp",
    )


def _strip_claude_hipcortex_block(text: str) -> tuple[str, bool]:
    """Remove hipcortex registration from CLAUDE.md body.

    Returns (new_text, removed). Prefer exact install-shaped substring.
    Fallback bounds the block to next H1 or heading+paragraph (never EOF wipe).
    """
    # Exact install registration (proactive variant first — longer / more specific)
    for reg in (_proactive_claude_registration(), _CLAUDE_REGISTRATION):
        if reg in text:
            return text.replace(reg, "", 1), True
        body = reg.lstrip("\n")
        if body and text.startswith(body):
            return text[len(body) :], True
        if body and body in text:
            return text.replace(body, "", 1), True

    # Fallback: locate heading marker
    marker = "\n# hipcortex"
    if marker in text:
        idx = text.index(marker)
    elif text.startswith("# hipcortex"):
        idx = 0
    else:
        stripped = text.lstrip()
        if stripped.startswith("# hipcortex"):
            idx = len(text) - len(stripped)
        else:
            return text, False

    next_h1 = text.find("\n# ", idx + 1)
    if next_h1 != -1:
        end = next_h1
    else:
        # No next H1: strip heading + contiguous non-empty body lines only
        # (max 10 body lines). Never delete to EOF blindly.
        rest = text[idx:]
        lines = rest.splitlines(keepends=True)
        end_rel = 0
        i = 0
        body_lines = 0
        # optional leading newline from "\n# hipcortex"
        if lines and lines[0] == "\n":
            end_rel += len(lines[0])
            i = 1
        # heading line
        if i < len(lines):
            end_rel += len(lines[i])
            i += 1
        while i < len(lines) and body_lines < 10:
            line = lines[i]
            if line.strip() == "":
                end_rel += len(line)
                break
            # stop before another heading-like line
            if line.startswith("# "):
                break
            end_rel += len(line)
            i += 1
            body_lines += 1
        end = idx + end_rel

    return text[:idx] + text[end:], True


def _uninstall_claude_code() -> bool:
    """Remove HipCortex skill from Claude Code. True if anything removed."""
    removed = False
    skill_dir = _skill_dir()
    if skill_dir.exists():
        shutil.rmtree(skill_dir)
        removed = True
    claude_md = Path.home() / ".claude" / "CLAUDE.md"
    if claude_md.exists():
        text = claude_md.read_text(encoding="utf-8")
        new_text, block_removed = _strip_claude_hipcortex_block(text)
        if block_removed:
            claude_md.write_text(new_text, encoding="utf-8")
            removed = True
    return removed

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


def _install_cursor(server_url: str, global_: bool = False) -> str:
    """Write/update .cursor/mcp.json. Returns created|updated|unchanged|skipped."""
    mcp_path = _cursor_mcp_path(global_=global_)
    if mcp_path is None:
        return INSTALL_SKIPPED
    return _write_mcp_servers(mcp_path, server_url)


def _vscode_settings_path() -> Path:
    if platform.system() == "Windows":
        return Path(os.environ.get("APPDATA", "")) / "Code" / "User" / "settings.json"
    if platform.system() == "Darwin":
        return Path.home() / "Library" / "Application Support" / "Code" / "User" / "settings.json"
    return (
        Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
        / "Code"
        / "User"
        / "settings.json"
    )


def _install_vscode(server_url: str) -> str:
    """Write mcpServers to VS Code settings.json. Returns install status."""
    settings_path = _vscode_settings_path()
    if not settings_path.exists():
        return INSTALL_SKIPPED  # VS Code not installed

    try:
        settings: dict = json.loads(settings_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, PermissionError):
        return INSTALL_SKIPPED

    entry = _desired_mcp_entry(server_url)
    mcp_servers = settings.setdefault("mcpServers", {})
    prev = mcp_servers.get("hipcortex")
    if prev == entry:
        return INSTALL_UNCHANGED

    had_entry = prev is not None
    mcp_servers["hipcortex"] = entry
    settings_path.write_text(json.dumps(settings, indent=2), encoding="utf-8")
    return INSTALL_UPDATED if had_entry else INSTALL_CREATED

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

def _windsurf_base() -> Path:
    if platform.system() == "Windows":
        return Path(os.environ.get("APPDATA", Path.home())) / "Codeium" / "windsurf"
    if platform.system() == "Darwin":
        return Path.home() / "Library" / "Application Support" / "Codeium" / "windsurf"
    return (
        Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
        / "Codeium"
        / "windsurf"
    )


def _install_windsurf(server_url: str) -> str:
    """Write/update Windsurf MCP config. Returns install status."""
    base = _windsurf_base()
    if not base.exists():
        return INSTALL_SKIPPED

    return _write_mcp_servers(base / "mcp_settings.json", server_url)


def _install_mcp_generic(server_url: str, mcp_path: Path) -> str:
    """Write MCP config to an arbitrary path (Cline, RooCode, etc.). Returns status."""
    return _write_mcp_servers(mcp_path, server_url)


def _install_cursor_prefer_local(server_url: str) -> str:
    """Install Cursor MCP: prefer project .cursor/mcp.json, else global."""
    local = _install_cursor(server_url, global_=False)
    # Local always can create parent dirs → never skipped. Keep fallback for safety.
    if local != INSTALL_SKIPPED:
        return local
    return _install_cursor(server_url, global_=True)


def _uninstall_cursor() -> bool:
    """Remove hipcortex from project + global Cursor mcp.json."""
    removed = False
    local = _cursor_mcp_path(global_=False)
    if local and _remove_mcp_entry(local):
        removed = True
    global_path = _cursor_mcp_path(global_=True)
    if global_path and _remove_mcp_entry(global_path):
        removed = True
    return removed


def _uninstall_windsurf() -> bool:
    return _remove_mcp_entry(_windsurf_base() / "mcp_settings.json")


def _uninstall_vscode() -> bool:
    return _remove_mcp_entry(_vscode_settings_path())


def _uninstall_mcp_path(mcp_path: Path) -> bool:
    return _remove_mcp_entry(mcp_path)


# ─── Antigravity / Hermes / OpenClaw MCP hosts ────────────────────────────────

def _antigravity_mcp_path() -> Path:
    return Path.home() / ".gemini" / "antigravity" / "mcp_config.json"


def _install_antigravity(server_url: str) -> str:
    """Write ~/.gemini/antigravity/mcp_config.json (mcpServers shape, like Cursor)."""
    try:
        return _write_mcp_servers(_antigravity_mcp_path(), server_url)
    except PermissionError:
        return INSTALL_SKIPPED


def _uninstall_antigravity() -> bool:
    return _remove_mcp_entry(_antigravity_mcp_path())


def _hermes_hipcortex_yaml(entry: dict) -> str:
    """2-space-indented hipcortex block under mcp_servers (no PyYAML)."""
    cmd = json.dumps(entry["command"])
    arg0 = json.dumps(entry["args"][0])
    url = json.dumps(entry["env"]["HIPCORTEX_URL"])
    return (
        f"  hipcortex:\n"
        f"    command: {cmd}\n"
        f"    args: [{arg0}]\n"
        f"    env:\n"
        f"      HIPCORTEX_URL: {url}\n"
    )


def _hermes_entry_matches(text: str, entry: dict) -> bool:
    """True if config.yaml already has hipcortex with same command/args/url.

    Accepts raw paths or json.dumps-escaped forms (Windows backslashes).
    """
    if "hipcortex:" not in text:
        return False

    def _present(val: str) -> bool:
        return val in text or json.dumps(val) in text

    return (
        _present(entry["command"])
        and _present(entry["args"][0])
        and _present(entry["env"]["HIPCORTEX_URL"])
    )


def _hermes_merge_yaml(text: str, entry: dict) -> str:
    """Insert or replace  hipcortex: under mcp_servers in YAML text."""
    import re

    block = _hermes_hipcortex_yaml(entry)
    # Replace existing hipcortex sub-key (2-space indent, body at 4+ spaces)
    # Callable repl avoids re.sub backslash mangling of Windows paths in block.
    if re.search(r"(?m)^  hipcortex:\s*$", text):
        return re.sub(
            r"(?m)^  hipcortex:\s*\n(?:[ ]{4,}.*\n)*",
            lambda _m: block,
            text,
            count=1,
        )
    if re.search(r"(?m)^mcp_servers:\s*$", text) or re.search(
        r"(?m)^mcp_servers:\s+", text
    ):
        return re.sub(
            r"(?m)^(mcp_servers:\s*\n)",
            lambda m: m.group(1) + block,
            text,
            count=1,
        )
    # No mcp_servers key — append section
    return text.rstrip() + "\n\nmcp_servers:\n" + block


def _install_hermes(server_url: str) -> str:
    """Merge mcp_servers.hipcortex into ~/.hermes/config.yaml (no PyYAML dep).

    Skips if ~/.hermes does not exist (user must install Hermes first).
    """
    hermes_dir = Path.home() / ".hermes"
    if not hermes_dir.is_dir():
        return INSTALL_SKIPPED

    path = hermes_dir / "config.yaml"
    entry = _desired_mcp_entry(server_url)
    block = _hermes_hipcortex_yaml(entry)

    if not path.exists():
        path.write_text("mcp_servers:\n" + block, encoding="utf-8")
        return INSTALL_CREATED

    text = path.read_text(encoding="utf-8")
    if _hermes_entry_matches(text, entry):
        return INSTALL_UNCHANGED

    had = "hipcortex:" in text
    new_text = _hermes_merge_yaml(text, entry)
    path.write_text(new_text if new_text.endswith("\n") else new_text + "\n", encoding="utf-8")
    return INSTALL_UPDATED if had else INSTALL_CREATED


def _uninstall_hermes() -> bool:
    """Remove hipcortex block from ~/.hermes/config.yaml."""
    import re

    path = Path.home() / ".hermes" / "config.yaml"
    if not path.exists():
        return False
    text = path.read_text(encoding="utf-8")
    if not re.search(r"(?m)^  hipcortex:\s*$", text):
        return False
    new = re.sub(
        r"(?m)^  hipcortex:\s*\n(?:[ ]{4,}.*\n)*",
        "",
        text,
        count=1,
    )
    path.write_text(new, encoding="utf-8")
    return True


def _openclaw_config_path() -> Path:
    env = os.environ.get("OPENCLAW_CONFIG_PATH")
    if env:
        return Path(env)
    return Path.home() / ".openclaw" / "openclaw.json"


def _install_openclaw(server_url: str) -> str:
    """Merge mcp.servers.hipcortex into ~/.openclaw/openclaw.json.

    Skips if ~/.openclaw missing (unless OPENCLAW_CONFIG_PATH is set).
    On JSON5/parse failure: write sidecar + print `openclaw mcp add` hint.
    """
    path = _openclaw_config_path()
    custom = bool(os.environ.get("OPENCLAW_CONFIG_PATH"))
    if not custom and not path.parent.is_dir():
        return INSTALL_SKIPPED

    entry = _desired_mcp_entry(server_url)
    oc_entry = {
        "command": entry["command"],
        "args": list(entry["args"]),
        "env": dict(entry["env"]),
    }

    data: dict = {}
    if path.exists():
        raw = path.read_text(encoding="utf-8")
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            # JSON5 / comments — cannot merge safely
            sidecar = path.parent / "openclaw.hipcortex.mcp.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            sidecar.write_text(
                json.dumps({"hipcortex": oc_entry}, indent=2) + "\n",
                encoding="utf-8",
            )
            arg0 = oc_entry["args"][0]
            url = oc_entry["env"]["HIPCORTEX_URL"]
            cmd = oc_entry["command"]
            print(
                f"  {_GRAY}OpenClaw config is not plain JSON (JSON5?). "
                f"Wrote sidecar {sidecar.name}. Merge manually or run:{_RESET}"
            )
            print(
                f"  openclaw mcp add hipcortex --command {cmd} "
                f"--arg {arg0} --env HIPCORTEX_URL={url}"
            )
            return INSTALL_CREATED

    if not isinstance(data, dict):
        data = {}
    mcp = data.setdefault("mcp", {})
    if not isinstance(mcp, dict):
        mcp = {}
        data["mcp"] = mcp
    servers = mcp.setdefault("servers", {})
    if not isinstance(servers, dict):
        servers = {}
        mcp["servers"] = servers

    prev = servers.get("hipcortex")
    if prev == oc_entry:
        return INSTALL_UNCHANGED

    had = prev is not None
    servers["hipcortex"] = oc_entry
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return INSTALL_UPDATED if had else INSTALL_CREATED


def _uninstall_openclaw() -> bool:
    """Remove mcp.servers.hipcortex from openclaw.json (+ sidecar if present)."""
    removed = False
    path = _openclaw_config_path()
    sidecar = path.parent / "openclaw.hipcortex.mcp.json"
    if sidecar.exists():
        try:
            sidecar.unlink()
            removed = True
        except OSError:
            pass
    if not path.exists():
        return removed
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return removed
    if not isinstance(data, dict):
        return removed
    mcp = data.get("mcp")
    if not isinstance(mcp, dict):
        return removed
    servers = mcp.get("servers")
    if not isinstance(servers, dict) or "hipcortex" not in servers:
        return removed
    del servers["hipcortex"]
    mcp["servers"] = servers
    data["mcp"] = mcp
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return True


def _grok_config_path() -> Path:
    """Grok Build config: GROK_CONFIG_PATH or ~/.grok/config.toml."""
    env = os.environ.get("GROK_CONFIG_PATH")
    if env:
        return Path(env)
    return Path.home() / ".grok" / "config.toml"


def _desired_grok_toml_block(server_url: str) -> str:
    """Canonical [mcp_servers.hipcortex] table for ~/.grok/config.toml."""
    entry = _desired_mcp_entry(server_url)
    cmd = json.dumps(entry["command"])
    arg0 = json.dumps(entry["args"][0])
    url = json.dumps(entry["env"]["HIPCORTEX_URL"])
    return (
        "[mcp_servers.hipcortex]\n"
        f"command = {cmd}\n"
        f"args = [{arg0}]\n"
        f"env = {{ HIPCORTEX_URL = {url} }}\n"
        "enabled = true\n"
    )


def _strip_grok_hipcortex_tables(text: str) -> str:
    """Remove [mcp_servers.hipcortex] and nested [mcp_servers.hipcortex.*] tables."""
    import re

    return re.sub(
        r"(?ms)^\[mcp_servers\.hipcortex(?:\.[^\]]*)?\][ \t]*\n(?:(?!^\[).*\n?)*",
        "",
        text,
    )


def _grok_entry_matches(text: str, entry: dict) -> bool:
    """True if config already has hipcortex with same command/args/url."""
    if "[mcp_servers.hipcortex]" not in text:
        return False
    try:
        import tomllib
    except ImportError:  # pragma: no cover — py<3.11
        tomllib = None  # type: ignore[assignment]
    if tomllib is not None:
        try:
            data = tomllib.loads(text)
        except Exception:
            data = None
        else:
            # Successful parse: trust structured match only (no substring fallthrough).
            if not isinstance(data, dict):
                return False
            servers = data.get("mcp_servers")
            if not isinstance(servers, dict):
                return False
            srv = servers.get("hipcortex")
            if not isinstance(srv, dict):
                return False
            args = srv.get("args") or []
            env = srv.get("env") or {}
            if not isinstance(env, dict):
                env = {}
            return (
                srv.get("command") == entry["command"]
                and (args[0] if args else None) == entry["args"][0]
                and env.get("HIPCORTEX_URL") == entry["env"]["HIPCORTEX_URL"]
            )
    # Fallback (no tomllib or parse failure): JSON-escaped string forms
    return (
        json.dumps(entry["command"]) in text
        and json.dumps(entry["args"][0]) in text
        and json.dumps(entry["env"]["HIPCORTEX_URL"]) in text
    )


def _install_grok(server_url: str) -> str:
    """Merge [mcp_servers.hipcortex] into ~/.grok/config.toml (stdlib only).

    Skips if ~/.grok missing unless GROK_CONFIG_PATH is set.
    """
    path = _grok_config_path()
    custom = bool(os.environ.get("GROK_CONFIG_PATH"))
    if not custom and not path.parent.is_dir():
        return INSTALL_SKIPPED

    entry = _desired_mcp_entry(server_url)
    block = _desired_grok_toml_block(server_url)

    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(block if block.endswith("\n") else block + "\n", encoding="utf-8")
        return INSTALL_CREATED

    text = path.read_text(encoding="utf-8")
    if _grok_entry_matches(text, entry):
        return INSTALL_UNCHANGED

    had = "[mcp_servers.hipcortex]" in text
    stripped = _strip_grok_hipcortex_tables(text)
    # Collapse excess trailing blank lines before append
    new_text = stripped.rstrip() + "\n\n" + block
    if not new_text.endswith("\n"):
        new_text += "\n"
    path.write_text(new_text, encoding="utf-8")
    return INSTALL_UPDATED if had else INSTALL_CREATED


def _uninstall_grok() -> bool:
    """Remove [mcp_servers.hipcortex] (+ nested) from Grok config.toml."""
    path = _grok_config_path()
    if not path.exists():
        return False
    text = path.read_text(encoding="utf-8")
    if "[mcp_servers.hipcortex" not in text:
        return False
    new = _strip_grok_hipcortex_tables(text)
    if new == text:
        return False
    # Keep file tidy: single trailing newline
    path.write_text(new.rstrip() + "\n" if new.strip() else "", encoding="utf-8")
    return True


def _resolve_uninstall_channels(args: argparse.Namespace) -> list[str]:
    """Channels to uninstall. Default --all when neither flag given (compat)."""
    channels = list(getattr(args, "channel", None) or [])
    all_flag = getattr(args, "all", False) is True
    if all_flag or not channels:
        return list(KNOWN_UNINSTALL_CHANNELS)
    # Validate + de-dupe preserving order (aliases → canonical)
    seen: set[str] = set()
    out: list[str] = []
    for ch in channels:
        key = ch.strip().lower()
        key = _UNINSTALL_CHANNEL_ALIASES.get(key, key)
        if key not in KNOWN_UNINSTALL_CHANNELS:
            print(f"  {_GRAY}–{_RESET} unknown channel '{ch}' (skip)")
            continue
        if key not in seen:
            seen.add(key)
            out.append(key)
    return out


def _uninstall_channel(channel: str) -> bool:
    """Uninstall a single channel. Returns True if something removed."""
    if channel == "claude-code":
        return _uninstall_claude_code()
    if channel == "cursor":
        return _uninstall_cursor()
    if channel == "windsurf":
        return _uninstall_windsurf()
    if channel == "vscode":
        return _uninstall_vscode()
    if channel == "cline":
        return _uninstall_mcp_path(Path.cwd() / ".cline" / "mcp.json")
    if channel == "roocode":
        return _uninstall_mcp_path(Path.cwd() / ".roo" / "mcp.json")
    if channel == "antigravity":
        return _uninstall_antigravity()
    if channel == "hermes":
        return _uninstall_hermes()
    if channel == "openclaw":
        return _uninstall_openclaw()
    if channel == "grok":
        return _uninstall_grok()
    return False


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
# ─── Framework starter file generator ────────────────────────────────────────

def _write_framework_starter(filename: str, code: str) -> tuple:
    """Write a starter integration file to cwd. Returns (success, path)."""
    dest = Path.cwd() / filename
    dest.write_text(code, encoding="utf-8")
    return True, str(dest)


# Framework id → package template under install/templates/
_FRAMEWORK_TEMPLATE_FILES = {
    "langchain": "langchain.py.tmpl",
    "crewai": "crewai.py.tmpl",
    "autogen": "autogen.py.tmpl",
    "llamaindex": "llamaindex.py.tmpl",
    "pydantic-ai": "pydantic_ai.py.tmpl",
    "dspy": "dspy.py.tmpl",
    "n8n": "n8n_curl.sh.tmpl",
}


def _templates_dir() -> Path:
    """Directory of package-shipped scaffold templates."""
    return Path(__file__).resolve().parent / "install" / "templates"


def _load_framework_template(framework_id: str) -> Optional[str]:
    """Load install/templates/{id}.tmpl text, or None if missing."""
    fname = _FRAMEWORK_TEMPLATE_FILES.get(framework_id)
    if not fname:
        return None
    path = _templates_dir() / fname
    if not path.is_file():
        return None
    return path.read_text(encoding="utf-8")


def _resolve_framework_code(
    framework_id: str,
    server_url: str,
    inline_fallback: Optional[str] = None,
) -> str:
    """Prefer package template; fall back to inline code string.

    Templates use ``{{SERVER_URL}}``; substituted with *server_url*.
    """
    text = _load_framework_template(framework_id)
    if text is None:
        text = inline_fallback or ""
    return text.replace("{{SERVER_URL}}", server_url)


def _detect_frameworks() -> set:
    """Detect installed Python frameworks by scanning requirements files."""
    found = set()
    for fname in ("requirements.txt", "pyproject.toml", "setup.py", "Pipfile"):
        p = Path.cwd() / fname
        if p.exists():
            text = p.read_text(encoding="utf-8", errors="ignore").lower()
            for fw in ("langchain", "crewai", "autogen", "llama_index", "llama-index",
                       "haystack", "flowise", "dspy"):
                if fw in text:
                    found.add(fw.replace("-", "_").replace("llama_index", "llama-index"))
    return found


def _build_agent_registry(server_url: str, python_exe: str) -> list:
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    detected = _detect_frameworks()

    def _detected_suffix(fw: str) -> str:
        return f" {_GREEN}[detected]{_RESET}" if fw in detected else ""

    return [
        # ── SECTION: Coding Assistants ──────────────────────────────────────
        {"id": "_section_ide", "name": "── Coding Assistants ─────────────────────────", "desc": "", "type": "section"},
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
            "fn": lambda: (_install_cursor_prefer_local(server_url), ".cursor/mcp.json"),
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
            "fn": lambda: (
                _install_mcp_generic(server_url, Path.cwd() / ".cline" / "mcp.json"),
                ".cline/mcp.json",
            ),
        },
        {
            "id": "roocode",
            "name": "RooCode",
            "desc": "RooVeterinary · .roo/mcp.json in project",
            "type": "mcp",
            "fn": lambda: (
                _install_mcp_generic(server_url, Path.cwd() / ".roo" / "mcp.json"),
                ".roo/mcp.json",
            ),
        },
        {
            "id": "antigravity",
            "name": "Antigravity",
            "desc": "Google Gemini · ~/.gemini/antigravity MCP",
            "type": "mcp",
            "fn": lambda: (
                _install_antigravity(server_url),
                "~/.gemini/antigravity/mcp_config.json",
            ),
        },
        {
            "id": "hermes",
            "name": "Hermes",
            "desc": "Nous · mcp_servers in ~/.hermes/config.yaml",
            "type": "mcp",
            "fn": lambda: (
                _install_hermes(server_url),
                "~/.hermes/config.yaml",
            ),
        },
        {
            "id": "openclaw",
            "name": "OpenClaw",
            "desc": "OpenClaw · mcp.servers in openclaw.json",
            "type": "mcp",
            "fn": lambda: (
                _install_openclaw(server_url),
                "~/.openclaw/openclaw.json",
            ),
        },
        {
            "id": "grok-build",
            "name": "Grok Build",
            "desc": "xAI · [mcp_servers.hipcortex] in ~/.grok/config.toml",
            "type": "mcp",
            "fn": lambda: (
                _install_grok(server_url),
                "~/.grok/config.toml",
            ),
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
        # ── SECTION: Agent Frameworks ────────────────────────────────────────
        {"id": "_section_fw", "name": "── Agent Frameworks ──────────────────────────", "desc": "", "type": "section"},
        {
            "id": "langchain",
            "name": f"LangChain{_detected_suffix('langchain')}",
            "desc": "drop-in ConversationBufferMemory replacement",
            "type": "framework",
            "file": "hipcortex_langchain.py",
            # Prefer install/templates/langchain.py.tmpl; inline is emergency fallback
            "code": _resolve_framework_code(
                "langchain",
                server_url,
                '''"""HipCortex + LangChain — package API starter.
from hipcortex.langchain_memory import HipCortexMemory
memory = HipCortexMemory.from_settings()
# memory = HipCortexMemory(session_id="my-agent", url="{{SERVER_URL}}")
''',
            ),
        },
        {
            "id": "crewai",
            "name": f"CrewAI{_detected_suffix('crewai')}",
            "desc": "RememberTool + RecallTool for crew agents",
            "type": "framework",
            "file": "hipcortex_crewai.py",
            "code": _resolve_framework_code(
                "crewai",
                server_url,
                '''"""HipCortex + CrewAI — package API starter.
from hipcortex.adapters.crewai import make_memory_tools
memory_tools = make_memory_tools()
# make_memory_tools(client=HipCortexClient("{{SERVER_URL}}"), agent_id="my-crew")
''',
            ),
        },
        {
            "id": "autogen",
            "name": f"AutoGen{_detected_suffix('autogen')}",
            "desc": "AutoGen 0.4 Memory protocol",
            "type": "framework",
            "file": "hipcortex_autogen.py",
            "code": _resolve_framework_code(
                "autogen",
                server_url,
                '''"""HipCortex + AutoGen — package API starter.
from hipcortex.adapters.autogen import HipCortexAutoGenMemory
memory = HipCortexAutoGenMemory.from_settings()
# HipCortexAutoGenMemory(client=HipCortexClient("{{SERVER_URL}}"), agent_id="my-agent")
''',
            ),
        },
        {
            "id": "llamaindex",
            "name": f"LlamaIndex{_detected_suffix('llama-index')}",
            "desc": "SimpleChatStore-compatible memory",
            "type": "framework",
            "file": "hipcortex_llamaindex.py",
            "code": _resolve_framework_code(
                "llamaindex",
                server_url,
                '''"""HipCortex + LlamaIndex — package API starter.
from hipcortex.adapters.common import client_from_settings
from hipcortex.llamaindex_storage import HipCortexChatStore, HipCortexStorageContext
client = client_from_settings()  # or HipCortexClient("{{SERVER_URL}}")
chat_store = HipCortexChatStore(client=client)
storage_context = HipCortexStorageContext(client=client)
''',
            ),
        },
        {
            "id": "pydantic-ai",
            "name": "Pydantic AI",
            "desc": "tool-use memory via REST client",
            "type": "framework",
            "file": "hipcortex_pydantic_ai.py",
            "code": _resolve_framework_code(
                "pydantic-ai",
                server_url,
                '''"""HipCortex + Pydantic AI — package API starter.
from hipcortex.adapters.common import client_from_settings
client = client_from_settings()  # or HipCortexClient("{{SERVER_URL}}")
def remember(text: str, actor: str = "default") -> str:
    return str(client.remember(text, actor=actor).get("record_id", "?"))
def recall(query: str, actor: str = "default") -> list:
    return client.recall(query, actor=actor)
''',
            ),
        },
        {
            "id": "flowise",
            "name": "Flowise / Dify",
            "desc": "no-code · paste OpenAPI URL into tool builder",
            "type": "guide",
            "guide": f"{server_url}/openapi.json  →  paste into Flowise/Dify Custom Tool",
        },
        {
            "id": "n8n",
            "name": "n8n / Make.com",
            "desc": "workflow automation · HTTP Request node",
            "type": "framework",
            "file": "hipcortex_n8n_curl.sh",
            "code": _resolve_framework_code(
                "n8n",
                server_url,
                '''#!/usr/bin/env bash
# HipCortex n8n/Make HTTP snippets
HC="{{SERVER_URL}}"
curl -X POST "$HC/memory/ingest" -H "Content-Type: application/json" \\
  -d '{"text": "User Alice upgraded to Pro tier"}'
curl "$HC/memory/search-flat?query=Alice+plan&limit=5"
''',
            ),
        },
        {
            "id": "dspy",
            "name": f"DSPy{_detected_suffix('dspy')}",
            "desc": "trace storage for DSPy compilation",
            "type": "framework",
            "file": "hipcortex_dspy.py",
            "code": _resolve_framework_code(
                "dspy",
                server_url,
                '''"""HipCortex + DSPy — package API starter.
from hipcortex.adapters.common import client_from_settings
client = client_from_settings()  # or HipCortexClient("{{SERVER_URL}}")
def save_trace(program_name: str, trace: dict, score: float):
    client.remember(
        f"DSPy trace for {program_name}: score={score:.3f}",
        actor=f"dspy-{program_name}",
        context="decision",
    )
def load_best_traces(program_name: str, limit: int = 5) -> list:
    return client.recall(f"DSPy trace {program_name} score", actor=f"dspy-{program_name}")
''',
            ),
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
    """Interactive multi-select. Skips section headers. Returns selected agent dicts."""
    import re as _re
    selectable = [i for i, a in enumerate(agents) if a["type"] != "section"]
    n_all = len(agents)
    selected: set = set()
    sel_cursor = 0

    def _display_name(a: dict) -> str:
        return _re.sub(r'\033\[[^m]*m', '', a["name"])

    # On Windows, cursor-up ANSI codes corrupt the display.
    # Use cls-style redraw: print a blank separator then full list.
    _is_windows = platform.system() == "Windows"

    def render():
        cur_real = selectable[sel_cursor] if selectable else -1
        if _is_windows:
            # Windows: no cursor-up — just reprint with blank line separator
            print()
        else:
            # Unix: move cursor up to overwrite previous render
            print(f"\033[{n_all + 3}A", end="")
        print(f"  {_BOLD}Select what to configure:{_RESET} {_GRAY}(Space toggle · Enter confirm · q quit){_RESET}")
        print()
        for i, agent in enumerate(agents):
            if agent["type"] == "section":
                print(f"  {_DIM}{agent['name']}{_RESET}")
                continue
            is_sel = i in selected
            is_cur = i == cur_real
            bullet  = f"{_CYAN}●{_RESET}" if is_sel else f"{_GRAY}○{_RESET}"
            raw_name = _display_name(agent)
            name_str = f"{_BOLD}{agent['name']}{_RESET}" if is_cur else agent["name"]
            desc    = f"{_DIM}{agent['desc']}{_RESET}"
            if agent["type"] == "guide":
                tag = f" {_CYAN}[guide]{_RESET}"
            elif agent["type"] == "framework":
                tag = f" {_CYAN}[starter file]{_RESET}"
            else:
                tag = ""
            prefix = "  ›" if is_cur else "   "
            pad = max(1, 20 - len(raw_name))
            print(f"{prefix} {bullet} {name_str}{' ' * pad}{desc}{tag}")
        count = len(selected)
        print(f"\n  {_GREEN}{count} selected{_RESET}" if count else f"\n  {_GRAY}none selected{_RESET}")

    # Initial draw
    if not _is_windows:
        # Reserve lines for cursor-up redraw on Unix
        print(f"  {_BOLD}Select what to configure:{_RESET}")
        print()
        for agent in agents:
            if agent["type"] == "section":
                print(f"  {_DIM}{agent['name']}{_RESET}")
            else:
                print(f"   ○ {_display_name(agent):<20} {_DIM}{agent['desc']}{_RESET}")
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
            sel_cursor = (sel_cursor - 1) % len(selectable)
        elif key == "down":
            sel_cursor = (sel_cursor + 1) % len(selectable)
        elif key == "space":
            real_i = selectable[sel_cursor]
            if real_i in selected:
                selected.discard(real_i)
            else:
                selected.add(real_i)
        elif key == "enter":
            break

        render()

    print()
    return [agents[i] for i in sorted(selected)]


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
        # Non-interactive: configure all auto-configurable agents
        # Framework starters only materialize when --scaffold (still listed if scaffold)
        chosen = [a for a in agents if a["type"] != "guide"]
        print(f"  --yes: configuring all {len(chosen)} supported agents\n")
    else:
        print(f"  {_BOLD}Which AI coding assistants do you use?{_RESET}\n")
        chosen = _run_wizard(agents)

    if not chosen:
        print(f"  {_GRAY}Nothing selected. Run 'hipcortex install' again anytime.{_RESET}\n")
        return

    # ── 4. Configure chosen agents ────────────────────────────────────────────
    print(f"\n  {_BOLD}{'Would configure' if dry_run else 'Configuring'}:{_RESET}\n")
    guide_items = []

    import re as _re
    framework_files = []
    status_counts: dict[str, int] = {
        INSTALL_CREATED: 0,
        INSTALL_UPDATED: 0,
        INSTALL_UNCHANGED: 0,
        INSTALL_SKIPPED: 0,
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
            guide_items.append(agent)
            print(f"  {_CYAN}ℹ{_RESET} {name:<22} {agent['guide']}")
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
    if guide_items:
        print(f"\n  {_DIM}Setup guides:{_RESET}")
        for a in guide_items:
            raw = _re.sub(r'\033\[[^m]*m', '', a["name"])
            print(f"    {raw}: {a['guide']}")
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
    data_dir = getattr(args, "data_dir", None) or os.environ.get(
        "DATA_DIR", str(d.DEFAULT_DATA_DIR)
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
    data_dir = getattr(args, "data_dir", None) or os.environ.get(
        "DATA_DIR", str(d.DEFAULT_DATA_DIR)
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
        {"id": "vscode-extension", "name": "VS Code VSIX", "status": "native", "install": "hipcortex-memory-0.5.4.vsix"},
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
