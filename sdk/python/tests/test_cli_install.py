"""Tests for hipcortex install CLI — run: pytest sdk/python/tests/test_cli_install.py -v"""
import argparse
import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch


def _make_fake_home(tmp_path: Path) -> Path:
    """Create a fake home directory with .claude/ present (simulates Claude Code installed)."""
    home = tmp_path / "home"
    (home / ".claude").mkdir(parents=True)
    return home


def test_install_claude_code_writes_skill_md(tmp_path):
    """install_claude_code writes SKILL.md to ~/.claude/skills/hipcortex/."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, _install_claude_code
        result = _install_claude_code("http://localhost:3030")

    assert result == INSTALL_CREATED
    skill_file = home / ".claude" / "skills" / "hipcortex" / "SKILL.md"
    assert skill_file.exists(), "SKILL.md not written"
    content = skill_file.read_text(encoding="utf-8")
    assert "HipCortex" in content
    assert "localhost:3030" in content


def test_install_claude_code_appends_registration(tmp_path):
    """install_claude_code appends registration line to CLAUDE.md."""
    home = _make_fake_home(tmp_path)
    claude_md = home / ".claude" / "CLAUDE.md"
    claude_md.write_text("# existing content\n")

    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _install_claude_code
        _install_claude_code("http://localhost:3030")

    content = claude_md.read_text()
    assert "hipcortex" in content
    assert "existing content" in content  # original preserved


def test_install_claude_code_no_duplicate(tmp_path):
    """Running install twice does not duplicate the registration entry."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _install_claude_code
        _install_claude_code("http://localhost:3030")
        _install_claude_code("http://localhost:3030")  # second call

    claude_md = home / ".claude" / "CLAUDE.md"
    content = claude_md.read_text()
    # Registration block itself contains ~6 "hipcortex" occurrences — ensure it's only written once
    assert content.count("# hipcortex") == 1


def test_install_claude_code_not_installed_returns_skipped(tmp_path):
    """Returns skipped when ~/.claude/ does not exist (Claude Code not installed)."""
    home = tmp_path / "home_no_claude"
    home.mkdir()
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_SKIPPED, _install_claude_code
        result = _install_claude_code("http://localhost:3030")
    assert result == INSTALL_SKIPPED


def test_install_cursor_writes_mcp_json(tmp_path):
    """install_cursor writes .cursor/mcp.json in the specified directory."""
    project_dir = tmp_path / "my-project"
    project_dir.mkdir()

    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home), \
         patch("hipcortex.cli.Path.cwd", return_value=project_dir):
        from hipcortex.cli import INSTALL_CREATED, _install_cursor
        result = _install_cursor("http://localhost:3030", global_=False)

    assert result == INSTALL_CREATED
    mcp_json = project_dir / ".cursor" / "mcp.json"
    assert mcp_json.exists()
    config = json.loads(mcp_json.read_text())
    assert "hipcortex" in config["mcpServers"]
    assert config["mcpServers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == "http://localhost:3030"


def test_install_claude_code_proactive_mode_writes_harness(tmp_path):
    """--mode proactive writes substrate-first SKILL with MUST + Harness and updates CLAUDE reg."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, _install_claude_code
        result = _install_claude_code("http://localhost:3030", mode="proactive")

    assert result == INSTALL_CREATED
    skill_file = home / ".claude" / "skills" / "hipcortex" / "SKILL.md"
    content = skill_file.read_text(encoding="utf-8")
    assert "You are a memory-centric agent" in content
    assert "MUST: Before any question involving project state" in content
    assert "Harness: Action space = MCP tools" in content
    assert "80-99%+ reduction" in content

    claude_md = home / ".claude" / "CLAUDE.md"
    reg = claude_md.read_text(encoding="utf-8")
    assert "Proactive substrate-first memory (Claude Agent Harness)" in reg or "MUST search/get_live_beliefs" in reg


def test_install_cursor_merges_existing_config(tmp_path):
    """install_cursor preserves existing mcpServers entries."""
    project_dir = tmp_path / "project"
    cursor_dir = project_dir / ".cursor"
    cursor_dir.mkdir(parents=True)
    existing = {"mcpServers": {"other-tool": {"command": "python", "args": ["other.py"]}}}
    (cursor_dir / "mcp.json").write_text(json.dumps(existing))

    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home), \
         patch("hipcortex.cli.Path.cwd", return_value=project_dir):
        from hipcortex.cli import _install_cursor
        _install_cursor("http://localhost:3030", global_=False)

    config = json.loads((cursor_dir / "mcp.json").read_text())
    assert "other-tool" in config["mcpServers"]  # preserved
    assert "hipcortex" in config["mcpServers"]   # added


def test_uninstall_removes_skill_dir(tmp_path):
    """uninstall_claude_code removes ~/.claude/skills/hipcortex/."""
    home = _make_fake_home(tmp_path)
    skill_dir = home / ".claude" / "skills" / "hipcortex"
    skill_dir.mkdir(parents=True)
    (skill_dir / "SKILL.md").write_text("content")

    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_claude_code
        _uninstall_claude_code()

    assert not skill_dir.exists()


def test_detect_platform_returns_tuple():
    """_detect_platform returns valid (os_name, arch) tuple."""
    from hipcortex.cli import _detect_platform
    os_name, arch = _detect_platform()
    assert os_name in ("linux", "macos", "windows")
    assert arch in ("amd64", "arm64")


def test_binary_url_format():
    """_binary_url returns correct GitHub release URL."""
    from hipcortex.cli import _binary_url
    url = _binary_url("linux", "arm64")
    assert "hipcortex-linux-arm64" in url
    assert "github.com/farmountain/HipCortex" in url

    url_win = _binary_url("windows", "amd64")
    assert url_win.endswith(".exe")


def test_install_claude_code_actor_configuration(tmp_path):
    """install_claude_code overrides default actor description in SKILL.md when actor is provided."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, _install_claude_code
        result = _install_claude_code("http://localhost:3030", actor="developer_alice")

    assert result == INSTALL_CREATED
    skill_file = home / ".claude" / "skills" / "hipcortex" / "SKILL.md"
    content = skill_file.read_text(encoding="utf-8")
    assert 'Use "developer_alice" as the actor.' in content
    assert 'Use the current git repository name as the actor' not in content


def test_cmd_install_writes_project_config(tmp_path, monkeypatch):
    """cmd_install writes .hipcortex/config.toml with url, actor, mode, channels."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (True, "ok"),
        },
        {
            "id": "cursor",
            "name": "Cursor",
            "desc": "",
            "type": "mcp",
            "fn": lambda: (True, "ok"),
        },
    ]
    args = MagicMock()
    args.url = "http://install:9090"
    args.force = False
    args.yes = True
    args.mode = "proactive"
    args.actor = "install-actor"
    args.dry_run = False
    args.scaffold = False

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents):
        from hipcortex.cli import cmd_install

        cmd_install(args)

    from hipcortex.config import load_settings

    s = load_settings(proj)
    assert s.url == "http://install:9090"
    assert s.actor == "install-actor"
    assert s.mode == "proactive"
    assert s.channels == ["claude-code", "cursor"]
    assert (proj / ".hipcortex" / "config.toml").is_file()


def _install_args(**kwargs):
    """Namespace-like args for cmd_install tests (real bools, not MagicMock attrs)."""
    defaults = {
        "url": "http://localhost:3030",
        "force": False,
        "yes": True,
        "mode": "conservative",
        "actor": None,
        "dry_run": False,
        "scaffold": False,
    }
    defaults.update(kwargs)
    return argparse.Namespace(**defaults)


def test_scaffold_off_by_default_no_framework_files(tmp_path, monkeypatch, capsys):
    """Default install must not write hipcortex_*.py (or other) framework starters to cwd."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)

    fw_code = "# starter"
    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (True, "ok"),
        },
        {
            "id": "langchain",
            "name": "LangChain",
            "desc": "",
            "type": "framework",
            "file": "hipcortex_langchain.py",
            "code": fw_code,
        },
    ]
    args = _install_args(url="http://x:1", scaffold=False)

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents), patch(
        "hipcortex.cli._write_framework_starter"
    ) as write_fw:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    write_fw.assert_not_called()
    assert not (proj / "hipcortex_langchain.py").exists()
    out = capsys.readouterr().out
    assert "--scaffold" in out or "package API only" in out


def test_scaffold_on_writes_framework_starter(tmp_path, monkeypatch):
    """--scaffold writes framework starter files to cwd."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    agents = [
        {
            "id": "langchain",
            "name": "LangChain",
            "desc": "",
            "type": "framework",
            "file": "hipcortex_langchain.py",
            "code": "# langchain starter\n",
        },
    ]
    args = _install_args(url="http://x:1", scaffold=True)

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents):
        from hipcortex.cli import cmd_install

        cmd_install(args)

    starter = proj / "hipcortex_langchain.py"
    assert starter.is_file()
    assert "langchain starter" in starter.read_text(encoding="utf-8")


def test_dry_run_no_writes(tmp_path, monkeypatch, capsys):
    """--dry-run prints plan; no MCP, skill, config, scaffold, or binary download."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    skill_calls = []
    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: skill_calls.append("called") or (True, "ok"),
        },
        {
            "id": "langchain",
            "name": "LangChain",
            "desc": "",
            "type": "framework",
            "file": "hipcortex_langchain.py",
            "code": "# no write",
        },
    ]
    args = _install_args(url="http://x:1", dry_run=True, scaffold=True)

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ) as mcp, patch(
        "hipcortex.cli._build_agent_registry", return_value=agents
    ), patch(
        "hipcortex.cli._write_framework_starter"
    ) as write_fw, patch(
        "hipcortex.cli._download_binary"
    ) as dl:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    mcp.assert_not_called()
    write_fw.assert_not_called()
    dl.assert_not_called()
    assert skill_calls == []
    assert not (proj / ".hipcortex" / "config.toml").exists()
    assert not (proj / "hipcortex_langchain.py").exists()
    out = capsys.readouterr().out
    assert "[dry-run]" in out
    assert "would install" in out.lower() or "would write" in out.lower()


def test_dry_run_would_download_when_no_url(tmp_path, monkeypatch, capsys):
    """--dry-run without --url plans binary download but does not download."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (True, "ok"),
        },
    ]
    args = _install_args(url=None, dry_run=True, force=True)

    fake_bin = tmp_path / "bin" / "hipcortex"
    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents), patch(
        "hipcortex.cli._detect_platform", return_value=("linux", "amd64")
    ), patch(
        "hipcortex.cli._binary_path", return_value=fake_bin
    ), patch(
        "hipcortex.cli._download_binary"
    ) as dl:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    dl.assert_not_called()
    out = capsys.readouterr().out
    assert "would download binary" in out


def test_non_tty_auto_enables_yes(tmp_path, monkeypatch, capsys):
    """Non-TTY without --yes auto-enables --yes and prints notice."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (True, "ok"),
        },
    ]
    args = _install_args(url="http://x:1", yes=False)

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents), patch(
        "hipcortex.cli.sys.stdin.isatty", return_value=False
    ), patch(
        "hipcortex.cli._run_wizard"
    ) as wizard:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    wizard.assert_not_called()
    assert args.yes is True
    out = capsys.readouterr().out
    assert "auto-enabling --yes" in out
    assert (proj / ".hipcortex" / "config.toml").is_file()


def test_parser_has_scaffold_and_dry_run():
    """build_parser exposes --scaffold and --dry-run on install."""
    from hipcortex.cli import build_parser

    p = build_parser()
    ns = p.parse_args(["install", "--scaffold", "--dry-run", "--yes"])
    assert ns.scaffold is True
    assert ns.dry_run is True
    assert ns.yes is True

    ns2 = p.parse_args(["install"])
    assert ns2.scaffold is False
    assert ns2.dry_run is False


def test_framework_templates_load_from_package():
    """install/templates/*.tmpl load; {{SERVER_URL}} substituted."""
    from hipcortex.cli import (
        _FRAMEWORK_TEMPLATE_FILES,
        _load_framework_template,
        _resolve_framework_code,
        _templates_dir,
    )

    assert _templates_dir().is_dir()
    for fid in ("langchain", "crewai", "autogen", "llamaindex", "pydantic-ai", "dspy", "n8n"):
        raw = _load_framework_template(fid)
        assert raw is not None, f"missing template for {fid}"
        assert "{{SERVER_URL}}" in raw or "from_settings" in raw or "client_from_settings" in raw or "make_memory_tools" in raw
        resolved = _resolve_framework_code(fid, "http://tmpl-test:3030")
        assert "{{SERVER_URL}}" not in resolved
        assert "http://tmpl-test:3030" in resolved or "from_settings" in resolved or "make_memory_tools" in resolved

    # Unknown id → empty unless fallback provided
    assert _load_framework_template("nope") is None
    assert _resolve_framework_code("nope", "http://x", "FALLBACK {{SERVER_URL}}") == "FALLBACK http://x"
    assert set(_FRAMEWORK_TEMPLATE_FILES) >= {
        "langchain", "crewai", "autogen", "llamaindex", "pydantic-ai", "dspy", "n8n"
    }


def test_scaffold_uses_package_template_content(tmp_path, monkeypatch):
    """--scaffold with real registry writes from_settings / make_memory_tools starters."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    # Real registry (not mocked) so package templates apply
    args = _install_args(url="http://scaffold-url:1", scaffold=True)
    # Restrict to framework agents only via patched yes path that already selects all
    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._install_claude_code", return_value="created"), patch(
        "hipcortex.cli._install_cursor_prefer_local", return_value="created"
    ), patch(
        "hipcortex.cli._install_windsurf", return_value="created"
    ), patch(
        "hipcortex.cli._install_vscode", return_value="created"
    ), patch(
        "hipcortex.cli._install_mcp_generic", return_value="created"
    ), patch(
        "hipcortex.cli._install_antigravity", return_value="created"
    ), patch(
        "hipcortex.cli._install_hermes", return_value="created"
    ), patch(
        "hipcortex.cli._install_openclaw", return_value="created"
    ):
        from hipcortex.cli import cmd_install

        cmd_install(args)

    lc = proj / "hipcortex_langchain.py"
    assert lc.is_file()
    text = lc.read_text(encoding="utf-8")
    assert "from_settings" in text
    assert "HipCortexMemory" in text

    crew = proj / "hipcortex_crewai.py"
    assert crew.is_file()
    assert "make_memory_tools" in crew.read_text(encoding="utf-8")

    n8n = proj / "hipcortex_n8n_curl.sh"
    assert n8n.is_file()
    assert "http://scaffold-url:1" in n8n.read_text(encoding="utf-8")


# ─── Phase 5B: idempotent statuses + channel uninstall ────────────────────────


def test_double_skill_install_second_unchanged(tmp_path):
    """Second identical skill install returns unchanged (no rewrite needed)."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import (
            INSTALL_CREATED,
            INSTALL_UNCHANGED,
            _install_claude_code,
        )

        first = _install_claude_code("http://localhost:3030")
        second = _install_claude_code("http://localhost:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    skill = home / ".claude" / "skills" / "hipcortex" / "SKILL.md"
    assert skill.is_file()


def test_double_cursor_install_second_unchanged(tmp_path):
    """Second identical Cursor MCP merge returns unchanged."""
    project_dir = tmp_path / "proj"
    project_dir.mkdir()
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli.Path.cwd", return_value=project_dir
    ):
        from hipcortex.cli import (
            INSTALL_CREATED,
            INSTALL_UNCHANGED,
            _install_cursor,
        )

        first = _install_cursor("http://localhost:3030", global_=False)
        second = _install_cursor("http://localhost:3030", global_=False)

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED


def test_cursor_install_updated_when_url_changes(tmp_path):
    """Different HIPCORTEX_URL → updated status."""
    project_dir = tmp_path / "proj"
    project_dir.mkdir()
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli.Path.cwd", return_value=project_dir
    ):
        from hipcortex.cli import INSTALL_CREATED, INSTALL_UPDATED, _install_cursor

        first = _install_cursor("http://localhost:3030", global_=False)
        second = _install_cursor("http://localhost:9999", global_=False)

    assert first == INSTALL_CREATED
    assert second == INSTALL_UPDATED
    cfg = json.loads((project_dir / ".cursor" / "mcp.json").read_text())
    assert cfg["mcpServers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == "http://localhost:9999"


def test_uninstall_channel_claude_code_only(tmp_path, monkeypatch, capsys):
    """uninstall --channel claude-code removes skill; leaves cursor MCP."""
    home = _make_fake_home(tmp_path)
    skill_dir = home / ".claude" / "skills" / "hipcortex"
    skill_dir.mkdir(parents=True)
    (skill_dir / "SKILL.md").write_text("skill", encoding="utf-8")
    (home / ".claude" / "CLAUDE.md").write_text(
        "# other\n\n# hipcortex\n- **hipcortex** skill\n",
        encoding="utf-8",
    )

    proj = tmp_path / "proj"
    cursor = proj / ".cursor"
    cursor.mkdir(parents=True)
    (cursor / "mcp.json").write_text(
        json.dumps(
            {
                "mcpServers": {
                    "hipcortex": {"command": "x", "args": []},
                    "other": {"command": "y"},
                }
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.chdir(proj)

    args = argparse.Namespace(channel=["claude-code"], all=False, purge=False)
    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli.INSTALL_DIR", home / ".hipcortex"
    ):
        from hipcortex.cli import cmd_uninstall

        cmd_uninstall(args)

    assert not skill_dir.exists()
    # Cursor MCP untouched
    cfg = json.loads((cursor / "mcp.json").read_text())
    assert "hipcortex" in cfg["mcpServers"]
    out = capsys.readouterr().out
    assert "Claude Code" in out


def test_uninstall_channel_cursor_removes_mcp_entry(tmp_path, monkeypatch):
    """uninstall --channel cursor drops hipcortex MCP key, keeps others."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    cursor = proj / ".cursor"
    cursor.mkdir(parents=True)
    (cursor / "mcp.json").write_text(
        json.dumps(
            {
                "mcpServers": {
                    "hipcortex": {"command": "x", "args": []},
                    "other-tool": {"command": "y"},
                }
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.chdir(proj)

    args = argparse.Namespace(channel=["cursor"], all=False, purge=False)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import cmd_uninstall

        cmd_uninstall(args)

    cfg = json.loads((cursor / "mcp.json").read_text())
    assert "hipcortex" not in cfg["mcpServers"]
    assert "other-tool" in cfg["mcpServers"]


def test_uninstall_default_all_channels(tmp_path, monkeypatch, capsys):
    """No --channel and no --all → default all known channels (compat)."""
    home = _make_fake_home(tmp_path)
    skill_dir = home / ".claude" / "skills" / "hipcortex"
    skill_dir.mkdir(parents=True)
    (skill_dir / "SKILL.md").write_text("x", encoding="utf-8")
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    args = argparse.Namespace(channel=None, all=False, purge=False)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import cmd_uninstall

        cmd_uninstall(args)

    assert not skill_dir.exists()
    out = capsys.readouterr().out
    assert "claude-code" in out
    assert "cursor" in out


def test_parser_uninstall_channel_and_all():
    """build_parser exposes --channel (append) and --all on uninstall."""
    from hipcortex.cli import build_parser

    p = build_parser()
    ns = p.parse_args(["uninstall", "--channel", "claude-code", "--channel", "cursor"])
    assert ns.channel == ["claude-code", "cursor"]
    assert ns.all is False
    assert ns.purge is False

    ns2 = p.parse_args(["uninstall", "--all", "--purge"])
    assert ns2.all is True
    assert ns2.purge is True
    assert ns2.channel is None


def test_cmd_install_prints_summary_counts(tmp_path, monkeypatch, capsys):
    """cmd_install prints Install summary with status counts."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)

    from hipcortex.cli import INSTALL_CREATED, INSTALL_UNCHANGED

    agents = [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (INSTALL_CREATED, "skill"),
        },
        {
            "id": "cursor",
            "name": "Cursor",
            "desc": "",
            "type": "mcp",
            "fn": lambda: (INSTALL_UNCHANGED, "mcp"),
        },
    ]
    args = _install_args(url="http://x:1")

    with patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch("hipcortex.cli._build_agent_registry", return_value=agents):
        from hipcortex.cli import cmd_install

        cmd_install(args)

    out = capsys.readouterr().out
    assert "Install summary:" in out
    assert "created" in out
    assert "unchanged" in out


# ─── Phase 6A: Antigravity / Hermes / OpenClaw ────────────────────────────────


def test_install_antigravity_writes_mcp_config(tmp_path):
    """Antigravity creates ~/.gemini/antigravity/mcp_config.json with mcpServers."""
    home = tmp_path / "home"
    home.mkdir()
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, INSTALL_UNCHANGED, _desired_mcp_entry, _install_antigravity

        first = _install_antigravity("http://127.0.0.1:3030")
        second = _install_antigravity("http://127.0.0.1:3030")
        expected = _desired_mcp_entry("http://127.0.0.1:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    mcp = home / ".gemini" / "antigravity" / "mcp_config.json"
    assert mcp.is_file()
    cfg = json.loads(mcp.read_text(encoding="utf-8"))
    assert cfg["mcpServers"]["hipcortex"] == expected


def test_uninstall_antigravity_removes_entry(tmp_path):
    home = tmp_path / "home"
    mcp_dir = home / ".gemini" / "antigravity"
    mcp_dir.mkdir(parents=True)
    (mcp_dir / "mcp_config.json").write_text(
        json.dumps(
            {
                "mcpServers": {
                    "hipcortex": {"command": "x", "args": []},
                    "other": {"command": "y"},
                }
            }
        ),
        encoding="utf-8",
    )
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_antigravity

        assert _uninstall_antigravity() is True

    cfg = json.loads((mcp_dir / "mcp_config.json").read_text(encoding="utf-8"))
    assert "hipcortex" not in cfg["mcpServers"]
    assert "other" in cfg["mcpServers"]


def test_install_hermes_skips_without_dir(tmp_path):
    home = tmp_path / "home"
    home.mkdir()
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_SKIPPED, _install_hermes

        assert _install_hermes("http://127.0.0.1:3030") == INSTALL_SKIPPED


def test_install_hermes_merges_config_yaml(tmp_path):
    home = tmp_path / "home"
    hermes = home / ".hermes"
    hermes.mkdir(parents=True)
    (hermes / "config.yaml").write_text(
        "model: gpt\nmcp_servers:\n  other:\n    command: echo\n",
        encoding="utf-8",
    )
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import (
            INSTALL_CREATED,
            INSTALL_UNCHANGED,
            _desired_mcp_entry,
            _install_hermes,
        )

        first = _install_hermes("http://127.0.0.1:3030")
        second = _install_hermes("http://127.0.0.1:3030")
        entry = _desired_mcp_entry("http://127.0.0.1:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    text = (hermes / "config.yaml").read_text(encoding="utf-8")
    assert "hipcortex:" in text
    assert "other:" in text  # preserved
    assert entry["args"][0] in text
    assert entry["env"]["HIPCORTEX_URL"] in text


def test_uninstall_hermes_removes_block(tmp_path):
    home = tmp_path / "home"
    hermes = home / ".hermes"
    hermes.mkdir(parents=True)
    (hermes / "config.yaml").write_text(
        "mcp_servers:\n  hipcortex:\n    command: python\n    args: [x]\n  keep:\n    command: y\n",
        encoding="utf-8",
    )
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_hermes

        assert _uninstall_hermes() is True

    text = (hermes / "config.yaml").read_text(encoding="utf-8")
    assert "hipcortex:" not in text
    assert "keep:" in text


def test_install_openclaw_skips_without_dir(tmp_path):
    home = tmp_path / "home"
    home.mkdir()
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_SKIPPED, _install_openclaw

        assert _install_openclaw("http://127.0.0.1:3030") == INSTALL_SKIPPED


def test_install_openclaw_merges_mcp_servers(tmp_path, monkeypatch):
    home = tmp_path / "home"
    oc = home / ".openclaw"
    oc.mkdir(parents=True)
    (oc / "openclaw.json").write_text(
        json.dumps({"mcp": {"servers": {"other": {"command": "z"}}}}),
        encoding="utf-8",
    )
    monkeypatch.delenv("OPENCLAW_CONFIG_PATH", raising=False)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import (
            INSTALL_CREATED,
            INSTALL_UNCHANGED,
            _desired_mcp_entry,
            _install_openclaw,
        )

        first = _install_openclaw("http://127.0.0.1:3030")
        second = _install_openclaw("http://127.0.0.1:3030")
        expected = _desired_mcp_entry("http://127.0.0.1:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    cfg = json.loads((oc / "openclaw.json").read_text(encoding="utf-8"))
    assert "other" in cfg["mcp"]["servers"]
    assert cfg["mcp"]["servers"]["hipcortex"] == expected


def test_install_openclaw_json5_fallback_sidecar(tmp_path, monkeypatch, capsys):
    """Non-JSON openclaw.json → sidecar + openclaw mcp add hint."""
    home = tmp_path / "home"
    oc = home / ".openclaw"
    oc.mkdir(parents=True)
    (oc / "openclaw.json").write_text(
        '{\n  // comment\n  "mcp": {}\n}\n',
        encoding="utf-8",
    )
    monkeypatch.delenv("OPENCLAW_CONFIG_PATH", raising=False)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, _install_openclaw

        status = _install_openclaw("http://127.0.0.1:3030")

    assert status == INSTALL_CREATED
    sidecar = oc / "openclaw.hipcortex.mcp.json"
    assert sidecar.is_file()
    out = capsys.readouterr().out
    assert "openclaw mcp add hipcortex" in out
    assert "HIPCORTEX_URL=" in out


def test_uninstall_openclaw_removes_server(tmp_path, monkeypatch):
    home = tmp_path / "home"
    oc = home / ".openclaw"
    oc.mkdir(parents=True)
    (oc / "openclaw.json").write_text(
        json.dumps(
            {
                "mcp": {
                    "servers": {
                        "hipcortex": {"command": "x", "args": []},
                        "keep": {"command": "y"},
                    }
                }
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.delenv("OPENCLAW_CONFIG_PATH", raising=False)
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_openclaw

        assert _uninstall_openclaw() is True

    cfg = json.loads((oc / "openclaw.json").read_text(encoding="utf-8"))
    assert "hipcortex" not in cfg["mcp"]["servers"]
    assert "keep" in cfg["mcp"]["servers"]


def test_registry_includes_phase6a_hosts():
    """_build_agent_registry lists antigravity, hermes, openclaw, grok-build."""
    from hipcortex.cli import _build_agent_registry

    agents = _build_agent_registry("http://127.0.0.1:3030", sys.executable)
    ids = {a["id"] for a in agents}
    assert {"antigravity", "hermes", "openclaw", "grok-build"} <= ids
    by_id = {a["id"]: a for a in agents}
    assert by_id["antigravity"]["type"] == "mcp"
    assert by_id["hermes"]["type"] == "mcp"
    assert by_id["openclaw"]["type"] == "mcp"
    assert by_id["grok-build"]["type"] == "guide"


def test_known_uninstall_channels_phase6a():
    from hipcortex.cli import KNOWN_UNINSTALL_CHANNELS

    for ch in ("antigravity", "hermes", "openclaw"):
        assert ch in KNOWN_UNINSTALL_CHANNELS

