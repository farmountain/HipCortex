"""HipCortex CLI — install command auto-configures AI coding assistants.

Usage:
    hipcortex install           # download binary + configure Claude Code + Cursor
    hipcortex install --url URL # use existing server instead of local binary
    hipcortex install --mode proactive  # substrate-first harness (MUST get_live_beliefs first etc)
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
DEFAULT_URL = "http://127.0.0.1:3030"
MANAGED_URL = "https://hipcortex.fly.dev"
PID_FILE = INSTALL_DIR / "hipcortex.pid"

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


def _install_claude_code(server_url: str, mode: str = "conservative") -> bool:
    """Write SKILL.md + append to CLAUDE.md. Returns True on success.
    Supports --mode proactive (uses substrate-first template + harness registration).
    """
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
    # If proactive, ensure the (now substrate-first) content; modify for emphasis if needed (minimal)
    if mode == "proactive":
        # Ensure key harness language is present (source is proactive base; this guards)
        if "MUST: Before any question" not in content:
            content = content.replace(
                "Invoke HipCortex when the user asks you to:",
                "You are a memory-centric agent. MUST search/get_live_beliefs first (substrate for state/hyp/pred); LLM only final/creative hyp.",
            )
    (skill_dir / "SKILL.md").write_text(content, encoding="utf-8")

    # Append registration to CLAUDE.md if not already present (update for proactive)
    claude_md = claude_dir / "CLAUDE.md"
    existing = claude_md.read_text(encoding="utf-8") if claude_md.exists() else ""
    if "hipcortex" not in existing:
        reg = _CLAUDE_REGISTRATION
        if mode == "proactive":
            reg = reg.replace(
                "Persistent memory for AI agents",
                "Proactive substrate-first memory (Claude Agent Harness) for AI agents. MUST search/get_live_beliefs first; use substrate for state/hyp/pred; LLM only final or creative hyp",
            )
        with claude_md.open("a", encoding="utf-8") as f:
            f.write(reg)

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
# ─── Framework starter file generator ────────────────────────────────────────

def _write_framework_starter(filename: str, code: str) -> tuple:
    """Write a starter integration file to cwd. Returns (success, path)."""
    dest = Path.cwd() / filename
    dest.write_text(code, encoding="utf-8")
    return True, str(dest)


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
        # ── SECTION: Agent Frameworks ────────────────────────────────────────
        {"id": "_section_fw", "name": "── Agent Frameworks ──────────────────────────", "desc": "", "type": "section"},
        {
            "id": "langchain",
            "name": f"LangChain{_detected_suffix('langchain')}",
            "desc": "drop-in ConversationBufferMemory replacement",
            "type": "framework",
            "file": "hipcortex_langchain.py",
            "code": f'''"""HipCortex + LangChain — drop-in memory integration.
Generated by: hipcortex install
Docs: https://github.com/farmountain/HipCortex
"""
from hipcortex.langchain_memory import HipCortexMemory, AsyncHipCortexMemory

# Sync usage (drop-in for ConversationBufferMemory)
memory = HipCortexMemory(
    session_id="my-agent",
    url="{server_url}",
    # api_key="sk-your-key",  # optional for managed tier
)

# In a LangChain chain:
# from langchain.chains import ConversationChain
# from langchain.chat_models import ChatOpenAI
# chain = ConversationChain(llm=ChatOpenAI(), memory=memory)
# chain.predict(input="Hello!")

# Async usage (FastAPI, Django async, LangChain 0.2+)
async def get_async_memory():
    from hipcortex import AsyncHipCortexClient
    from hipcortex.langchain_memory import AsyncHipCortexMemory
    client = AsyncHipCortexClient("{server_url}")
    return AsyncHipCortexMemory(client=client, session_id="my-agent")
''',
        },
        {
            "id": "crewai",
            "name": f"CrewAI{_detected_suffix('crewai')}",
            "desc": "RememberTool + RecallTool for crew agents",
            "type": "framework",
            "file": "hipcortex_crewai.py",
            "code": f'''"""HipCortex + CrewAI — memory tools for your crew.
Generated by: hipcortex install
Docs: https://github.com/farmountain/HipCortex
"""
from hipcortex import HipCortexClient
from hipcortex.adapters.crewai import (
    HipCortexRememberTool,
    HipCortexRecallTool,
    HipCortexForgetTool,
)

client = HipCortexClient("{server_url}")

memory_tools = [
    HipCortexRememberTool(client=client, agent_id="my-crew"),
    HipCortexRecallTool(client=client, agent_id="my-crew"),
    HipCortexForgetTool(client=client, agent_id="my-crew"),
]

# Add to any CrewAI agent:
# from crewai import Agent
# researcher = Agent(
#     role="Researcher",
#     goal="Research and remember findings",
#     tools=memory_tools,
#     verbose=True,
# )
''',
        },
        {
            "id": "autogen",
            "name": f"AutoGen{_detected_suffix('autogen')}",
            "desc": "AutoGen 0.4 Memory protocol",
            "type": "framework",
            "file": "hipcortex_autogen.py",
            "code": f'''"""HipCortex + AutoGen 0.4 — persistent memory for agents.
Generated by: hipcortex install
Docs: https://github.com/farmountain/HipCortex
"""
from hipcortex import HipCortexClient
from hipcortex.adapters.autogen import HipCortexAutoGenMemory

client = HipCortexClient("{server_url}")
memory = HipCortexAutoGenMemory(client=client, agent_id="my-agent", top_k=10)

# AutoGen 0.4: pass memory=[memory] to AssistantAgent
# from autogen_agentchat.agents import AssistantAgent
# agent = AssistantAgent(
#     name="assistant",
#     model_client=...,
#     memory=[memory],
# )

# AutoGen 0.3 legacy hooks:
# agent.register_hook("process_message_before_send", memory.on_message_sent_v03)
# agent.register_hook("process_all_messages_before_reply", memory.on_messages_received_v03)

# Zero-config shortcut:
# client.remember("We decided to use PostgreSQL")
# memories = client.recall("database choice")
''',
        },
        {
            "id": "llamaindex",
            "name": f"LlamaIndex{_detected_suffix('llama-index')}",
            "desc": "SimpleChatStore-compatible memory",
            "type": "framework",
            "file": "hipcortex_llamaindex.py",
            "code": f'''"""HipCortex + LlamaIndex — persistent chat memory.
Generated by: hipcortex install
Docs: https://github.com/farmountain/HipCortex
"""
from hipcortex import HipCortexClient
from hipcortex.llamaindex_storage import HipCortexChatStore, HipCortexStorageContext

client = HipCortexClient("{server_url}")

# Drop-in for SimpleChatStore
chat_store = HipCortexChatStore(client=client)

# Use with ChatMemoryBuffer:
# from llama_index.core.memory import ChatMemoryBuffer
# memory = ChatMemoryBuffer.from_defaults(
#     token_limit=3000,
#     chat_store=chat_store,
#     chat_store_key="user-42",
# )

# Storage context:
storage_context = HipCortexStorageContext.from_url("{server_url}")
''',
        },
        {
            "id": "pydantic-ai",
            "name": "Pydantic AI",
            "desc": "tool-use memory via REST client",
            "type": "framework",
            "file": "hipcortex_pydantic_ai.py",
            "code": f'''"""HipCortex + Pydantic AI — memory tools for AI agents.
Generated by: hipcortex install
Docs: https://github.com/farmountain/HipCortex
"""
from hipcortex import HipCortexClient
from pydantic_ai import Agent, Tool

client = HipCortexClient("{server_url}")

def remember(text: str, actor: str = "default") -> str:
    """Store a memory. Use for decisions, findings, constraints."""
    result = client.remember(text, actor=actor)
    return f"Stored: {{result.get('record_id', '?')}}"

def recall(query: str, actor: str = "default") -> list:
    """Recall relevant memories matching a query."""
    return client.recall(query, actor=actor)

# Register as tools:
# agent = Agent("openai:gpt-4o", tools=[remember, recall])
''',
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
            "code": f'''#!/usr/bin/env bash
# HipCortex + n8n / Make.com / Zapier — HTTP Request examples
# Generated by: hipcortex install

HC="{server_url}"

# Store memory (POST /memory/ingest — zero-config, auto-classifies)
curl -X POST "$HC/memory/ingest" \\
  -H "Content-Type: application/json" \\
  -d '{{"text": "User Alice upgraded to Pro tier"}}'

# Search memories (plain text response — easy to parse in n8n)
curl "$HC/memory/search-flat?query=Alice+plan&limit=5"

# In n8n: HTTP Request node → POST $HC/memory/ingest → body: {{"text": "{{{{$json.text}}}}"}}
# In Make.com: HTTP module → POST $HC/memory/ingest
# OpenAPI spec for auto-import: {server_url}/openapi.json
''',
        },
        {
            "id": "dspy",
            "name": f"DSPy{_detected_suffix('dspy')}",
            "desc": "trace storage for DSPy compilation",
            "type": "framework",
            "file": "hipcortex_dspy.py",
            "code": f'''"""HipCortex + DSPy — store compilation traces and optimization history.
Generated by: hipcortex install
"""
from hipcortex import HipCortexClient

client = HipCortexClient("{server_url}")

def save_trace(program_name: str, trace: dict, score: float):
    """Store a DSPy optimization trace."""
    client.remember(
        f"DSPy trace for {{program_name}}: score={{score:.3f}} — {{str(trace)[:200]}}",
        actor=f"dspy-{{program_name}}",
        context="decision",
    )

def load_best_traces(program_name: str, limit: int = 5) -> list:
    """Retrieve top traces for a program."""
    return client.recall(f"DSPy trace {{program_name}} score", actor=f"dspy-{{program_name}}")
''',
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

    # Support --mode proactive for claude (surgical: patch claude fn only; other fns unchanged)
    mode = getattr(args, "mode", "conservative")
    for a in agents:
        if a.get("id") == "claude-code":
            a["fn"] = lambda: (_install_claude_code(server_url, mode), "~/.claude/skills/hipcortex/")
            break

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

    import re as _re
    framework_files = []

    for agent in chosen:
        if agent["type"] == "section":
            continue
        name = _re.sub(r'\033\[[^m]*m', '', agent["name"])  # strip ANSI for display width
        if agent["type"] == "guide":
            guide_items.append(agent)
            print(f"  {_CYAN}ℹ{_RESET} {name:<22} {agent['guide']}")
            continue
        if agent["type"] == "framework":
            try:
                ok, dest = _write_framework_starter(agent["file"], agent["code"])
                print(f"  {_GREEN}✓{_RESET} {name:<22} {_DIM}{dest}{_RESET}")
                framework_files.append((name, dest))
            except Exception as e:
                print(f"  ✗ {name:<22} error: {e}")
            continue
        try:
            ok, detail = agent["fn"]()
            if ok:
                print(f"  {_GREEN}✓{_RESET} {name:<22} {_DIM}{detail}{_RESET}")
            else:
                print(f"  {_GRAY}–{_RESET} {name:<22} {_DIM}not found (install first){_RESET}")
        except Exception as e:
            print(f"  ✗ {name:<22} error: {e}")

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
    configured_names = {a["id"] for a in chosen if a["type"] not in ("guide", "section")}

    if "claude-code" in configured_names:
        print(f"  Claude Code  →  /hipcortex remember 'your note'")
        print(f"                  /hipcortex recall 'query'")
    if configured_names & {"cursor", "windsurf", "cline", "roocode", "vscode"}:
        print(f"  Cursor/etc   →  restart IDE · use hipcortex MCP tools")
    if framework_files:
        print(f"\n  {_BOLD}Starter files written:{_RESET}")
        for fname, fpath in framework_files:
            print(f"  {_GREEN}✓{_RESET} {fname:<20} {_DIM}{fpath}{_RESET}")
        print(f"  {_DIM}Edit these files and import into your project.{_RESET}")
    if guide_items:
        print(f"\n  {_DIM}Setup guides:{_RESET}")
        for a in guide_items:
            raw = _re.sub(r'\033\[[^m]*m', '', a["name"])
            print(f"    {raw}: {a['guide']}")
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
    health_url = f"http://127.0.0.1:{port}/health"

    # ── Check if something is already on this port ──────────────────────
    import socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(0.5)
    port_taken = sock.connect_ex(("127.0.0.1", port)) == 0
    sock.close()

    if port_taken:
        try:
            import json as _json, urllib.request as _ur
            with _ur.urlopen(health_url, timeout=2) as r:
                data = _json.loads(r.read())
                if data.get("service") == "hipcortex":
                    print(f"✓ HipCortex already running on http://127.0.0.1:{port}")
                    print(f"  Version: {data.get('version', 'unknown')}")
                    print(f"  Run 'hipcortex stop' to stop it first.")
                    sys.exit(1)
        except Exception:
            pass
        print(f"✗ Port {port} already in use by another application.")
        print(f"  Use --port to pick a different port.")
        sys.exit(1)

    # ── Check PID file for stale daemon ─────────────────────────────────
    if PID_FILE.exists():
        try:
            old_pid = int(PID_FILE.read_text().strip())
            import signal
            os.kill(old_pid, 0)  # check if alive
            print(f"✗ HipCortex daemon already running (PID {old_pid}).")
            print(f"  Run 'hipcortex stop' to stop it first.")
            sys.exit(1)
        except (OSError, ValueError):
            # PID file exists but process is dead — clean it up
            print(f"Cleaning up stale PID file (PID {old_pid} no longer running)")
            PID_FILE.unlink(missing_ok=True)

    data_dir = args.data_dir or os.environ.get("DATA_DIR", str(Path.home() / ".hipcortex" / "data"))
    Path(data_dir).mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["PORT"] = str(port)
    env["DATA_DIR"] = data_dir

    print(f"Starting HipCortex on http://127.0.0.1:{port}")
    print(f"Data: {data_dir}")

    import subprocess
    proc = subprocess.Popen([str(binary)], env=env)

    # Write PID file
    PID_FILE.write_text(str(proc.pid))
    print(f"PID: {proc.pid}")

    # Poll /health until server is ready (max 10 seconds)
    import time
    for _ in range(20):
        time.sleep(0.5)
        try:
            with urllib.request.urlopen(health_url, timeout=1) as r:
                import json as _json2
                if r.status == 200:
                    data = _json2.loads(r.read())
                    ver = data.get("version", "?")
                    print(f"✓ HipCortex v{ver} running on http://127.0.0.1:{port}")
                    print(f"  /hipcortex remember 'your note'   (Claude Code)")
                    print(f"  curl {health_url}")
                    print(f"  hipcortex stop   (to shut down)")
                    print()
                    break
        except Exception:
            pass
    else:
        print("  Server may still be starting... check health manually.")


def cmd_stop(args: argparse.Namespace) -> None:
    """Stop the local HipCortex server."""
    import signal

    port = args.port or int(os.environ.get("PORT", "3030"))

    # Try PID file first
    if PID_FILE.exists():
        try:
            pid = int(PID_FILE.read_text().strip())
            os.kill(pid, signal.SIGTERM)
            print(f"✓ Sent stop signal to HipCortex (PID {pid})")
            PID_FILE.unlink()
            # Wait for graceful shutdown
            import time
            for _ in range(10):
                time.sleep(0.3)
                try:
                    os.kill(pid, 0)
                except OSError:
                    print(f"  Process exited cleanly.")
                    return
            # Force kill if still alive
            try:
                os.kill(pid, signal.SIGKILL)
                print(f"  Force-killed after timeout.")
            except OSError:
                pass
            return
        except (ValueError, OSError) as e:
            print(f"PID file exists but process unreachable: {e}")
            PID_FILE.unlink(missing_ok=True)

    # Fallback: find and kill process on port
    import subprocess, platform
    try:
        if platform.system() == "Windows":
            result = subprocess.run(
                ["netstat", "-ano"], capture_output=True, text=True
            )
            for line in result.stdout.splitlines():
                if f":{port}" in line and "LISTENING" in line:
                    pid = line.strip().split()[-1]
                    subprocess.run(["taskkill", "/PID", pid, "/F"], capture_output=True)
                    print(f"✓ Stopped HipCortex on port {port} (PID {pid})")
                    PID_FILE.unlink(missing_ok=True)
                    return
        else:
            result = subprocess.run(
                ["lsof", "-ti", f":{port}"], capture_output=True, text=True
            )
            pids = result.stdout.strip().split()
            for pid in pids:
                os.kill(int(pid), signal.SIGTERM)
                print(f"✓ Stopped HipCortex on port {port} (PID {pid})")
            PID_FILE.unlink(missing_ok=True)
            return
    except Exception:
        pass

    print(f"No HipCortex process found on port {port}.")
    PID_FILE.unlink(missing_ok=True)


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
    p_install.add_argument("--mode", choices=["conservative", "proactive"], default="conservative", help="SKILL policy: conservative (explicit) or proactive (substrate-first harness; MUST search/get_live_beliefs first)")

    # start
    p_start = sub.add_parser("start", help="Start the local HipCortex server")
    p_start.add_argument("--port", type=int, help="Port (default: 3030)")
    p_start.add_argument("--data-dir", help="Data directory (default: ~/.hipcortex/data)")

    # status
    p_status = sub.add_parser("status", help="Check server health")
    p_status.add_argument("--url", help="Server URL to check")

    # stop
    p_stop = sub.add_parser("stop", help="Stop the local HipCortex server")
    p_stop.add_argument("--port", type=int, help="Port (default: 3030)")

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

    # index
    p_index = sub.add_parser("index", help="Index a codebase into the HipCortex knowledge graph")
    p_index.add_argument("path", nargs="?", default=".", help="Directory or file to index (default: current dir)")
    p_index.add_argument("--url", help="Server URL")
    p_index.add_argument("--actor", help="Actor label for this codebase (default: codebase)")
    p_index.add_argument("--extensions", help="Comma-separated file extensions (default: .py,.ts,.js)")

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
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
