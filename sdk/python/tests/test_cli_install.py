"""Tests for hipcortex install CLI — run: pytest sdk/python/tests/test_cli_install.py -v"""
import argparse
import json
import platform
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _install_claude_code
        _install_claude_code("http://localhost:3030")

    content = claude_md.read_text()
    assert "hipcortex" in content
    assert "existing content" in content  # original preserved


def test_install_claude_code_no_duplicate(tmp_path):
    """Running install twice does not duplicate the registration entry."""
    home = _make_fake_home(tmp_path)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_SKIPPED, _install_claude_code
        result = _install_claude_code("http://localhost:3030")
    assert result == INSTALL_SKIPPED


def test_install_cursor_writes_mcp_json(tmp_path):
    """install_cursor writes .cursor/mcp.json in the specified directory."""
    project_dir = tmp_path / "my-project"
    project_dir.mkdir()

    home = _make_fake_home(tmp_path)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), \
         patch("hipcortex.install_hosts.Path.cwd", return_value=project_dir), patch("hipcortex.cli.Path.cwd", return_value=project_dir):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), \
         patch("hipcortex.install_hosts.Path.cwd", return_value=project_dir), patch("hipcortex.cli.Path.cwd", return_value=project_dir):
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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
        "index": None,  # None = auto (proactive on / conservative off)
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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


def test_parser_has_index_flags():
    """build_parser exposes --index / --no-index; default None (mode-dependent)."""
    from hipcortex.cli import build_parser

    p = build_parser()
    ns = p.parse_args(["install"])
    assert ns.index is None

    ns_on = p.parse_args(["install", "--index"])
    assert ns_on.index is True

    ns_off = p.parse_args(["install", "--no-index"])
    assert ns_off.index is False


def test_resolve_install_index_flag_modes():
    """proactive defaults on; conservative off; flags override; dry_run off."""
    from hipcortex.cli import _resolve_install_index_flag

    assert _resolve_install_index_flag(
        argparse.Namespace(index=None), "proactive", dry_run=False
    ) is True
    assert _resolve_install_index_flag(
        argparse.Namespace(index=None), "conservative", dry_run=False
    ) is False
    assert _resolve_install_index_flag(
        argparse.Namespace(index=True), "conservative", dry_run=False
    ) is True
    assert _resolve_install_index_flag(
        argparse.Namespace(index=False), "proactive", dry_run=False
    ) is False
    assert _resolve_install_index_flag(
        argparse.Namespace(index=None), "proactive", dry_run=True
    ) is False
    # MagicMock auto-attr is not bool True/False → treated as auto
    assert _resolve_install_index_flag(
        MagicMock(index=MagicMock()), "proactive", dry_run=False
    ) is True
    assert _resolve_install_index_flag(
        MagicMock(index=MagicMock()), "conservative", dry_run=False
    ) is False


def _minimal_install_agents():
    return [
        {
            "id": "claude-code",
            "name": "Claude Code",
            "desc": "",
            "type": "native",
            "fn": lambda: (True, "ok"),
        },
    ]


def test_proactive_install_calls_index_bootstrap(tmp_path, monkeypatch):
    """mode=proactive (not dry_run) defaults to index bootstrap."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    (proj / "main.py").write_text("x=1\n", encoding="utf-8")
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)

    args = _install_args(url="http://idx:3030", mode="proactive", index=None)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch("hipcortex.cli._bootstrap_codebase_index", return_value=True) as boot:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_called_once()
    call_kw = boot.call_args
    assert call_kw[0][0] == "http://idx:3030" or call_kw.args[0] == "http://idx:3030"


def test_proactive_no_index_skips_bootstrap(tmp_path, monkeypatch):
    """--no-index skips bootstrap even in proactive mode."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    args = _install_args(url="http://x:1", mode="proactive", index=False)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch("hipcortex.cli._bootstrap_codebase_index") as boot:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_not_called()


def test_conservative_default_skips_index(tmp_path, monkeypatch):
    """conservative mode does not index unless --index."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    args = _install_args(url="http://x:1", mode="conservative", index=None)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch("hipcortex.cli._bootstrap_codebase_index") as boot:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_not_called()


def test_conservative_with_index_calls_bootstrap(tmp_path, monkeypatch):
    """conservative + --index forces bootstrap."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    args = _install_args(url="http://x:1", mode="conservative", index=True)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch("hipcortex.cli._bootstrap_codebase_index", return_value=True) as boot:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_called_once()


def test_dry_run_skips_index_even_proactive(tmp_path, monkeypatch, capsys):
    """dry_run never calls index; may print would-bootstrap for proactive."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    args = _install_args(url="http://x:1", mode="proactive", dry_run=True, index=None)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch("hipcortex.cli._bootstrap_codebase_index") as boot:
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_not_called()
    out = capsys.readouterr().out
    assert "would bootstrap codebase index" in out
    assert not (proj / ".hipcortex" / "config.toml").exists()


def test_index_bootstrap_failure_does_not_fail_install(tmp_path, monkeypatch, capsys):
    """Index errors are best-effort; install still completes + writes config."""
    home = _make_fake_home(tmp_path)
    proj = tmp_path / "proj"
    proj.mkdir()
    (proj / "app.py").write_text("def f(): pass\n", encoding="utf-8")
    monkeypatch.chdir(proj)

    from hipcortex import config as cfg

    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")

    args = _install_args(url="http://dead:9", mode="proactive", index=True)

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.cli._install_mcp_server"
    ), patch(
        "hipcortex.cli._build_agent_registry", return_value=_minimal_install_agents()
    ), patch(
        "hipcortex.cli._bootstrap_codebase_index",
        side_effect=RuntimeError("should be caught by cmd_install?"),
    ) as boot:
        # cmd_install should NOT wrap bootstrap — bootstrap itself never raises.
        # Simulate bootstrap swallowing: return False instead of raise.
        boot.side_effect = None
        boot.return_value = False
        from hipcortex.cli import cmd_install

        cmd_install(args)

    boot.assert_called_once()
    assert (proj / ".hipcortex" / "config.toml").is_file()
    out = capsys.readouterr().out
    assert "Ready!" in out


def test_bootstrap_codebase_index_swallows_errors(tmp_path, monkeypatch, capsys):
    """_bootstrap_codebase_index never raises; prints skip on indexer failure."""
    proj = tmp_path / "srcproj"
    proj.mkdir()
    (proj / "m.py").write_text("a=1\n", encoding="utf-8")

    with patch("hipcortex.cli._server_healthy", return_value=True), patch(
        "hipcortex.cli._looks_like_codebase", return_value=True
    ), patch("hipcortex.client.HipCortexClient"), patch(
        "hipcortex.indexer.CodeIndexer", side_effect=RuntimeError("index boom")
    ):
        from hipcortex.cli import _bootstrap_codebase_index

        ok = _bootstrap_codebase_index("http://localhost:3030", path=proj)

    assert ok is False
    assert "skipped" in capsys.readouterr().out


def test_bootstrap_codebase_index_uses_code_indexer(tmp_path, monkeypatch, capsys):
    """_bootstrap_codebase_index calls CodeIndexer.index when healthy + source tree."""
    proj = tmp_path / "srcproj"
    proj.mkdir()
    (proj / "m.py").write_text("a=1\n", encoding="utf-8")
    monkeypatch.chdir(proj)

    mock_indexer = MagicMock()
    mock_indexer.index.return_value = {"files": 1, "nodes": 2, "edges": 3}

    with patch("hipcortex.cli._server_healthy", return_value=True), patch(
        "hipcortex.client.HipCortexClient"
    ), patch("hipcortex.indexer.CodeIndexer", return_value=mock_indexer):
        from hipcortex.cli import _bootstrap_codebase_index

        ok = _bootstrap_codebase_index("http://localhost:3030", path=proj, actor="codebase")

    assert ok is True
    mock_indexer.index.assert_called_once()
    out = capsys.readouterr().out
    assert "1 files" in out or "files" in out


def test_bootstrap_skips_when_server_unhealthy(tmp_path, monkeypatch, capsys):
    proj = tmp_path / "srcproj"
    proj.mkdir()
    (proj / "m.py").write_text("a=1\n", encoding="utf-8")

    with patch("hipcortex.cli._server_healthy", return_value=False), patch(
        "hipcortex.indexer.CodeIndexer"
    ) as Idx:
        from hipcortex.cli import _bootstrap_codebase_index

        ok = _bootstrap_codebase_index("http://localhost:3030", path=proj)

    assert ok is False
    Idx.assert_not_called()
    assert "skipped" in capsys.readouterr().out


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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.install_hosts.Path.cwd", return_value=project_dir
    ), patch("hipcortex.cli.Path.cwd", return_value=project_dir):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
        "hipcortex.install_hosts.Path.cwd", return_value=project_dir
    ), patch("hipcortex.cli.Path.cwd", return_value=project_dir):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home), patch(
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_antigravity

        assert _uninstall_antigravity() is True

    cfg = json.loads((mcp_dir / "mcp_config.json").read_text(encoding="utf-8"))
    assert "hipcortex" not in cfg["mcpServers"]
    assert "other" in cfg["mcpServers"]


def test_install_hermes_skips_without_dir(tmp_path):
    home = tmp_path / "home"
    home.mkdir()
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    # Paths written via json.dumps (Windows backslashes) or raw
    assert entry["args"][0] in text or json.dumps(entry["args"][0]) in text
    assert entry["env"]["HIPCORTEX_URL"] in text or json.dumps(entry["env"]["HIPCORTEX_URL"]) in text


def test_uninstall_hermes_removes_block(tmp_path):
    home = tmp_path / "home"
    hermes = home / ".hermes"
    hermes.mkdir(parents=True)
    (hermes / "config.yaml").write_text(
        "mcp_servers:\n  hipcortex:\n    command: python\n    args: [x]\n  keep:\n    command: y\n",
        encoding="utf-8",
    )
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_hermes

        assert _uninstall_hermes() is True

    text = (hermes / "config.yaml").read_text(encoding="utf-8")
    assert "hipcortex:" not in text
    assert "keep:" in text


def test_install_openclaw_skips_without_dir(tmp_path):
    home = tmp_path / "home"
    home.mkdir()
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
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
    assert by_id["grok-build"]["type"] == "mcp"


def test_known_uninstall_channels_phase6a():
    from hipcortex.cli import KNOWN_UNINSTALL_CHANNELS

    for ch in ("antigravity", "hermes", "openclaw", "grok"):
        assert ch in KNOWN_UNINSTALL_CHANNELS


# ─── Grok Build (Phase 6c) ───────────────────────────────────────────────────


def test_install_grok_skips_without_dir(tmp_path, monkeypatch):
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_SKIPPED, _install_grok

        assert _install_grok("http://127.0.0.1:3030") == INSTALL_SKIPPED


def test_install_grok_writes_config_toml(tmp_path, monkeypatch):
    home = tmp_path / "home"
    grok = home / ".grok"
    grok.mkdir(parents=True)
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import (
            INSTALL_CREATED,
            INSTALL_UNCHANGED,
            _desired_mcp_entry,
            _install_grok,
        )

        first = _install_grok("http://127.0.0.1:3030")
        second = _install_grok("http://127.0.0.1:3030")
        entry = _desired_mcp_entry("http://127.0.0.1:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    text = (grok / "config.toml").read_text(encoding="utf-8")
    assert "[mcp_servers.hipcortex]" in text
    assert entry["command"] in text or json.dumps(entry["command"])[1:-1] in text
    assert entry["args"][0] in text or json.dumps(entry["args"][0])[1:-1] in text
    assert "HIPCORTEX_URL" in text
    assert "http://127.0.0.1:3030" in text
    assert "enabled = true" in text


def test_install_grok_preserves_other_servers(tmp_path, monkeypatch):
    home = tmp_path / "home"
    grok = home / ".grok"
    grok.mkdir(parents=True)
    (grok / "config.toml").write_text(
        "[mcp_servers.headroom]\n"
        'command = "headroom"\n'
        'args = ["mcp", "serve"]\n'
        "enabled = true\n"
        "\n"
        "[mcp_servers.kaggle]\n"
        'url = "https://www.kaggle.com/mcp"\n'
        "enabled = true\n",
        encoding="utf-8",
    )
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, _install_grok

        status = _install_grok("http://127.0.0.1:3030")

    assert status == INSTALL_CREATED
    text = (grok / "config.toml").read_text(encoding="utf-8")
    assert "[mcp_servers.headroom]" in text
    assert "headroom" in text
    assert "[mcp_servers.kaggle]" in text
    assert "kaggle.com" in text
    assert "[mcp_servers.hipcortex]" in text


def test_install_grok_updates_url(tmp_path, monkeypatch):
    home = tmp_path / "home"
    grok = home / ".grok"
    grok.mkdir(parents=True)
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, INSTALL_UPDATED, _install_grok

        first = _install_grok("http://127.0.0.1:3030")
        second = _install_grok("http://127.0.0.1:4040")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UPDATED
    text = (grok / "config.toml").read_text(encoding="utf-8")
    assert "http://127.0.0.1:4040" in text
    assert "http://127.0.0.1:3030" not in text


def test_uninstall_grok_removes_table(tmp_path, monkeypatch):
    home = tmp_path / "home"
    grok = home / ".grok"
    grok.mkdir(parents=True)
    (grok / "config.toml").write_text(
        "[mcp_servers.keep]\n"
        'command = "keep"\n'
        "\n"
        "[mcp_servers.hipcortex]\n"
        'command = "python"\n'
        'args = ["x"]\n'
        'env = { HIPCORTEX_URL = "http://127.0.0.1:3030" }\n'
        "enabled = true\n",
        encoding="utf-8",
    )
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _uninstall_grok

        assert _uninstall_grok() is True

    text = (grok / "config.toml").read_text(encoding="utf-8")
    assert "[mcp_servers.hipcortex]" not in text
    assert "[mcp_servers.keep]" in text
    assert "keep" in text


def test_install_grok_respects_GROK_CONFIG_PATH(tmp_path, monkeypatch):
    """Custom GROK_CONFIG_PATH works without ~/.grok under home."""
    home = tmp_path / "home"
    home.mkdir()
    custom = tmp_path / "custom" / "my-grok.toml"
    custom.parent.mkdir(parents=True)
    monkeypatch.setenv("GROK_CONFIG_PATH", str(custom))
    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_CREATED, INSTALL_UNCHANGED, _install_grok

        first = _install_grok("http://127.0.0.1:3030")
        second = _install_grok("http://127.0.0.1:3030")

    assert first == INSTALL_CREATED
    assert second == INSTALL_UNCHANGED
    assert custom.is_file()
    text = custom.read_text(encoding="utf-8")
    assert "[mcp_servers.hipcortex]" in text
    assert not (home / ".grok").exists()


# ─── P0 host config safety ───────────────────────────────────────────────────


def test_write_mcp_servers_corrupt_json_does_not_wipe(tmp_path, capsys):
    """Corrupt MCP JSON must not be overwritten; original bytes preserved + sidecar."""
    mcp = tmp_path / "mcp.json"
    corrupt = (
        '{\n  // comment\n  "mcpServers": {"other": {"command": "x", "args": []}}\n}\n'
    )
    mcp.write_text(corrupt, encoding="utf-8")
    original = mcp.read_bytes()

    from hipcortex.cli import INSTALL_REFUSED, _write_mcp_servers

    status = _write_mcp_servers(mcp, "http://localhost:3030")
    assert status == INSTALL_REFUSED
    assert mcp.read_bytes() == original, "primary MCP file must not be wiped"
    # Must not become hipcortex-only valid JSON
    assert b"hipcortex" not in mcp.read_bytes()

    sidecar = tmp_path / "mcp.hipcortex.mcp.json"
    assert sidecar.is_file()
    side = json.loads(sidecar.read_text(encoding="utf-8"))
    assert "hipcortex" in side["mcpServers"]
    out = capsys.readouterr().out
    assert "corrupt" in out.lower() or "not" in out.lower()


def test_write_mcp_servers_mcpservers_not_dict_skips(tmp_path):
    """mcpServers present but not a dict → REFUSED, file unchanged."""
    mcp = tmp_path / "mcp.json"
    payload = json.dumps({"mcpServers": ["not", "a", "dict"], "keep": True})
    mcp.write_text(payload, encoding="utf-8")
    original = mcp.read_bytes()

    from hipcortex.cli import INSTALL_REFUSED, _write_mcp_servers

    status = _write_mcp_servers(mcp, "http://localhost:3030")
    assert status == INSTALL_REFUSED
    assert mcp.read_bytes() == original


def test_write_mcp_servers_happy_path_merge_and_create(tmp_path):
    """Happy path: create, merge other servers, unchanged when same entry."""
    mcp = tmp_path / "mcp.json"
    existing = {
        "mcpServers": {"other-tool": {"command": "python", "args": ["other.py"]}}
    }
    mcp.write_text(json.dumps(existing), encoding="utf-8")

    from hipcortex.cli import (
        INSTALL_CREATED,
        INSTALL_UNCHANGED,
        INSTALL_UPDATED,
        _desired_mcp_entry,
        _write_mcp_servers,
    )

    url = "http://localhost:3030"
    status = _write_mcp_servers(mcp, url)
    assert status == INSTALL_CREATED
    data = json.loads(mcp.read_text(encoding="utf-8"))
    assert "other-tool" in data["mcpServers"]
    assert data["mcpServers"]["hipcortex"] == _desired_mcp_entry(url)

    status2 = _write_mcp_servers(mcp, url)
    assert status2 == INSTALL_UNCHANGED

    status3 = _write_mcp_servers(mcp, "http://localhost:4040")
    assert status3 == INSTALL_UPDATED
    data3 = json.loads(mcp.read_text(encoding="utf-8"))
    assert data3["mcpServers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == (
        "http://localhost:4040"
    )
    assert "other-tool" in data3["mcpServers"]


def test_uninstall_claude_preserves_trailing_content(tmp_path):
    """Uninstall removes hipcortex block but keeps other sections after it."""
    home = _make_fake_home(tmp_path)
    skill_dir = home / ".claude" / "skills" / "hipcortex"
    skill_dir.mkdir(parents=True)
    (skill_dir / "SKILL.md").write_text("x", encoding="utf-8")

    from hipcortex.cli import _CLAUDE_REGISTRATION, _uninstall_claude_code

    claude_md = home / ".claude" / "CLAUDE.md"
    claude_md.write_text(
        "# existing\n\nfoo\n"
        + _CLAUDE_REGISTRATION
        + "\n# other section\nbar keeps going\n",
        encoding="utf-8",
    )

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        assert _uninstall_claude_code() is True

    content = claude_md.read_text(encoding="utf-8")
    assert "existing" in content
    assert "foo" in content
    assert "# other section" in content
    assert "bar keeps going" in content
    assert "# hipcortex" not in content
    assert not skill_dir.exists()


def test_uninstall_claude_no_next_h1_no_tail_wipe(tmp_path):
    """No next H1 after hipcortex must not wipe trailing prose to EOF."""
    home = _make_fake_home(tmp_path)

    from hipcortex.cli import _CLAUDE_REGISTRATION, _uninstall_claude_code

    claude_md = home / ".claude" / "CLAUDE.md"
    trailing = (
        "\n## notes (H2 not H1)\n"
        "Keep this prose after hipcortex with no next H1.\n"
        "Important trail content.\n"
    )
    claude_md.write_text(
        "# preamble\nstuff\n" + _CLAUDE_REGISTRATION + trailing,
        encoding="utf-8",
    )

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        assert _uninstall_claude_code() is True

    content = claude_md.read_text(encoding="utf-8")
    assert "preamble" in content
    assert "stuff" in content
    assert "Important trail content" in content
    assert "Keep this prose" in content
    assert "## notes" in content
    assert "# hipcortex" not in content


def test_install_cursor_prefer_local_refuses_corrupt_no_global_fallback(tmp_path, monkeypatch):
    """Corrupt project mcp.json returns REFUSED; must not silently write global."""
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.chdir(tmp_path)
    local = tmp_path / ".cursor" / "mcp.json"
    local.parent.mkdir(parents=True)
    local.write_text("{ not valid json", encoding="utf-8")
    original = local.read_bytes()

    # Global would be under home if we incorrectly fall through
    if platform.system() == "Windows":
        monkeypatch.setenv("APPDATA", str(home / "AppData" / "Roaming"))
        global_mcp = home / "AppData" / "Roaming" / "Cursor" / "User" / "mcp.json"
    else:
        monkeypatch.setenv("XDG_CONFIG_HOME", str(home / ".config"))
        global_mcp = home / ".config" / "Cursor" / "mcp.json"

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import INSTALL_REFUSED, _install_cursor_prefer_local

        status = _install_cursor_prefer_local("http://127.0.0.1:3030")

    assert status == INSTALL_REFUSED
    assert local.read_bytes() == original
    assert not global_mcp.exists(), "must not fall through to global on corrupt local"

