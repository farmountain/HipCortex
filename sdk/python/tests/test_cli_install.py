"""Tests for hipcortex install CLI — run: pytest sdk/python/tests/test_cli_install.py -v"""
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
        from hipcortex.cli import _install_claude_code
        result = _install_claude_code("http://localhost:3030")

    assert result is True
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


def test_install_claude_code_not_installed_returns_false(tmp_path):
    """Returns False when ~/.claude/ does not exist (Claude Code not installed)."""
    home = tmp_path / "home_no_claude"
    home.mkdir()
    with patch("hipcortex.cli.Path.home", return_value=home):
        from hipcortex.cli import _install_claude_code
        result = _install_claude_code("http://localhost:3030")
    assert result is False


def test_install_cursor_writes_mcp_json(tmp_path):
    """install_cursor writes .cursor/mcp.json in the specified directory."""
    project_dir = tmp_path / "my-project"
    project_dir.mkdir()

    home = _make_fake_home(tmp_path)
    with patch("hipcortex.cli.Path.home", return_value=home), \
         patch("hipcortex.cli.Path.cwd", return_value=project_dir):
        from hipcortex.cli import _install_cursor
        result = _install_cursor("http://localhost:3030", global_=False)

    assert result is True
    mcp_json = project_dir / ".cursor" / "mcp.json"
    assert mcp_json.exists()
    config = json.loads(mcp_json.read_text())
    assert "hipcortex" in config["mcpServers"]
    assert config["mcpServers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == "http://localhost:3030"


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
