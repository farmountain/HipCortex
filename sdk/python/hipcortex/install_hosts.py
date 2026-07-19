"""Host installers for HipCortex CLI (skill/MCP registration + uninstall).

Extracted from cli.py so host install/uninstall logic lives outside the entrypoint.
Symbols re-exported from hipcortex.cli for backward-compatible tests/imports.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import sys
from pathlib import Path
from typing import Optional

# --- ANSI colors (shared with cli prints) ---
_CYAN  = "\033[96m"
_GRAY  = "\033[90m"
_BOLD  = "\033[1m"
_DIM   = "\033[2m"
_GREEN = "\033[92m"
_RESET = "\033[0m"
_CLEAR_LINE = "\033[2K\r"

# --- Install constants ---
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
INSTALL_REFUSED = "refused"  # host config corrupt; primary not overwritten
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

# --- Skill / MCP / host installers ---
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

    Returns created|updated|unchanged|skipped|refused.
    On corrupt JSON or non-dict root/mcpServers: never overwrite primary;
    write sidecar ``{stem}.hipcortex.mcp.json`` and return INSTALL_REFUSED.
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
        return INSTALL_REFUSED

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
        reg = (
            _proactive_claude_registration()
            if mode == "proactive"
            else _CLAUDE_REGISTRATION
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
        appdata = os.environ.get("APPDATA") or str(Path.home() / "AppData" / "Roaming")
        return Path(appdata) / "Code" / "User" / "settings.json"
    if platform.system() == "Darwin":
        return Path.home() / "Library" / "Application Support" / "Code" / "User" / "settings.json"
    return (
        Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
        / "Code"
        / "User"
        / "settings.json"
    )


def _install_vscode(server_url: str) -> str:
    """Write mcpServers to VS Code settings.json. Returns install status.

    Missing settings.json → SKIPPED. Corrupt / non-dict root / non-dict
    mcpServers → REFUSED + sidecar; primary never wiped.
    """
    settings_path = _vscode_settings_path()
    if not settings_path.exists():
        return INSTALL_SKIPPED  # VS Code not installed

    entry = _desired_mcp_entry(server_url)

    def _refuse(reason: str) -> str:
        sidecar = settings_path.parent / "settings.hipcortex.mcp.json"
        settings_path.parent.mkdir(parents=True, exist_ok=True)
        sidecar.write_text(
            json.dumps({"mcpServers": {"hipcortex": entry}}, indent=2) + "\n",
            encoding="utf-8",
        )
        print(
            f"  {_GRAY}VS Code settings.json is corrupt ({reason}); "
            f"original not overwritten. Wrote sidecar {sidecar.name}.{_RESET}"
        )
        return INSTALL_REFUSED

    try:
        raw = settings_path.read_text(encoding="utf-8")
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        return _refuse("invalid JSON")
    except PermissionError:
        return INSTALL_SKIPPED

    if not isinstance(parsed, dict):
        return _refuse("root is not an object")
    if "mcpServers" in parsed and not isinstance(parsed["mcpServers"], dict):
        return _refuse("mcpServers is not an object")

    settings = parsed
    mcp_servers = settings.setdefault("mcpServers", {})
    prev = mcp_servers.get("hipcortex")
    if prev == entry:
        return INSTALL_UNCHANGED

    had_entry = prev is not None
    mcp_servers["hipcortex"] = entry
    _atomic_write_text(settings_path, json.dumps(settings, indent=2))
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
    """Install Cursor MCP: prefer project .cursor/mcp.json, else global.

    Fall through to global only when local is SKIPPED (path unavailable).
    REFUSED (corrupt local mcp.json) must not install global silently.
    """
    local = _install_cursor(server_url, global_=False)
    if local == INSTALL_SKIPPED:
        return _install_cursor(server_url, global_=True)
    return local


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


def _openclaw_refuse(path: Path, oc_entry: dict, reason: str) -> str:
    """Write openclaw sidecar only; never touch primary. Returns INSTALL_REFUSED."""
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
        f"  {_GRAY}OpenClaw config refuse ({reason}); "
        f"original not overwritten. Wrote sidecar {sidecar.name}. "
        f"Merge manually or run:{_RESET}"
    )
    print(
        f"  openclaw mcp add hipcortex --command {cmd} "
        f"--arg {arg0} --env HIPCORTEX_URL={url}"
    )
    return INSTALL_REFUSED


def _install_openclaw(server_url: str) -> str:
    """Merge mcp.servers.hipcortex into ~/.openclaw/openclaw.json.

    Skips if ~/.openclaw missing (unless OPENCLAW_CONFIG_PATH is set).
    On JSON5/parse failure or non-dict mcp/servers: REFUSED + sidecar;
    primary never wiped.
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
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            # JSON5 / comments — cannot merge safely; do not pretend create
            return _openclaw_refuse(path, oc_entry, "invalid JSON / JSON5")
        if not isinstance(parsed, dict):
            return _openclaw_refuse(path, oc_entry, "root is not an object")
        data = parsed
        if "mcp" in data and not isinstance(data["mcp"], dict):
            return _openclaw_refuse(path, oc_entry, "mcp is not an object")
        mcp_existing = data.get("mcp")
        if isinstance(mcp_existing, dict) and "servers" in mcp_existing:
            if not isinstance(mcp_existing["servers"], dict):
                return _openclaw_refuse(path, oc_entry, "mcp.servers is not an object")

    mcp = data.setdefault("mcp", {})
    if not isinstance(mcp, dict):
        # unreachable when path existed (refused above); for missing file
        return _openclaw_refuse(path, oc_entry, "mcp is not an object")
    servers = mcp.setdefault("servers", {})
    if not isinstance(servers, dict):
        return _openclaw_refuse(path, oc_entry, "mcp.servers is not an object")

    prev = servers.get("hipcortex")
    if prev == oc_entry:
        return INSTALL_UNCHANGED

    had = prev is not None
    servers["hipcortex"] = oc_entry
    _atomic_write_text(path, json.dumps(data, indent=2) + "\n")
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

