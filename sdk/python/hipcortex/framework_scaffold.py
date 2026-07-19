"""Framework scaffold + agent registry for HipCortex CLI install.

Extracted from cli.py so registry/templates live outside the entrypoint.
Symbols re-exported from hipcortex.cli for backward-compatible tests/imports.
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional

from .install_hosts import (
    _GREEN,
    _RESET,
    _install_antigravity,
    _install_claude_code,
    _install_cursor_prefer_local,
    _install_grok,
    _install_hermes,
    _install_mcp_generic,
    _install_openclaw,
    _install_vscode,
    _install_windsurf,
)

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
            for fw in (
                "langchain",
                "crewai",
                "autogen",
                "llama_index",
                "llama-index",
                "haystack",
                "flowise",
                "dspy",
            ):
                if fw in text:
                    found.add(
                        fw.replace("-", "_").replace("llama_index", "llama-index")
                    )
    return found


# Agent registry — id, display name, short description, type, install fn / guide URL
# type: "native" | "mcp" | "guide" | "framework" | "section"


def _build_agent_registry(server_url: str, python_exe: str) -> list:
    # python_exe retained for API stability (callers pass sys.executable)
    _ = python_exe
    mcp_server_py = str(Path.home() / ".hipcortex-mcp" / "server.py")
    _ = mcp_server_py  # reserved for future MCP path display
    detected = _detect_frameworks()

    def _detected_suffix(fw: str) -> str:
        return f" {_GREEN}[detected]{_RESET}" if fw in detected else ""

    return [
        # ── SECTION: Coding Assistants ──────────────────────────────────────
        {
            "id": "_section_ide",
            "name": "── Coding Assistants ─────────────────────────",
            "desc": "",
            "type": "section",
        },
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "Anthropic · SKILL.md native, no MCP process",
            "type": "native",
            "fn": lambda: (
                _install_claude_code(server_url),
                "~/.claude/skills/hipcortex/",
            ),
        },
        {
            "id": "cursor",
            "name": "Cursor",
            "desc": "Anysphere · MCP tools in AI panel",
            "type": "mcp",
            "fn": lambda: (
                _install_cursor_prefer_local(server_url),
                ".cursor/mcp.json",
            ),
        },
        {
            "id": "windsurf",
            "name": "Windsurf",
            "desc": "Codeium · global MCP settings",
            "type": "mcp",
            "fn": lambda: (
                _install_windsurf(server_url),
                "~/.codeium/windsurf/mcp_settings.json",
            ),
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
        {
            "id": "_section_fw",
            "name": "── Agent Frameworks ──────────────────────────",
            "desc": "",
            "type": "section",
        },
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
