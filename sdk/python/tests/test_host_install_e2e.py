"""Host installer E2E smoke — Antigravity / Hermes / OpenClaw / Grok.

Pure filesystem: isolated home via Path.home patch. No live hosts, no network.
Run: pytest sdk/python/tests/test_host_install_e2e.py -q --tb=line
"""
from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pytest

URL = "http://127.0.0.1:3030"


def _isolated_home(tmp_path: Path) -> Path:
    """Create fake home with all four host root dirs present."""
    home = tmp_path / "home"
    (home / ".gemini" / "antigravity").mkdir(parents=True)
    (home / ".hermes").mkdir(parents=True)
    (home / ".openclaw").mkdir(parents=True)
    (home / ".grok").mkdir(parents=True)
    return home


def _assert_hipcortex_and_url(text: str) -> None:
    assert "hipcortex" in text
    assert "HIPCORTEX_URL" in text
    assert URL in text


@pytest.mark.host_e2e
def test_all_hosts_install_idempotent_uninstall(tmp_path, monkeypatch):
    """Install all four → shapes OK → second install unchanged → uninstall clean."""
    home = _isolated_home(tmp_path)
    monkeypatch.delenv("OPENCLAW_CONFIG_PATH", raising=False)
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)

    from hipcortex.cli import (
        INSTALL_CREATED,
        INSTALL_UNCHANGED,
        _install_antigravity,
        _install_grok,
        _install_hermes,
        _install_openclaw,
        _uninstall_antigravity,
        _uninstall_grok,
        _uninstall_hermes,
        _uninstall_openclaw,
    )

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        # ── first install ──────────────────────────────────────────────
        results = {
            "antigravity": _install_antigravity(URL),
            "hermes": _install_hermes(URL),
            "openclaw": _install_openclaw(URL),
            "grok": _install_grok(URL),
        }
        assert all(r == INSTALL_CREATED for r in results.values()), results

        # ── config paths + content ─────────────────────────────────────
        antigravity_mcp = home / ".gemini" / "antigravity" / "mcp_config.json"
        hermes_yaml = home / ".hermes" / "config.yaml"
        openclaw_json = home / ".openclaw" / "openclaw.json"
        grok_toml = home / ".grok" / "config.toml"

        assert antigravity_mcp.is_file()
        assert hermes_yaml.is_file()
        assert openclaw_json.is_file()
        assert grok_toml.is_file()

        ag = json.loads(antigravity_mcp.read_text(encoding="utf-8"))
        assert "hipcortex" in ag.get("mcpServers", {})
        assert ag["mcpServers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == URL

        hermes_text = hermes_yaml.read_text(encoding="utf-8")
        _assert_hipcortex_and_url(hermes_text)
        assert "mcp_servers:" in hermes_text

        oc = json.loads(openclaw_json.read_text(encoding="utf-8"))
        assert "hipcortex" in oc.get("mcp", {}).get("servers", {})
        assert oc["mcp"]["servers"]["hipcortex"]["env"]["HIPCORTEX_URL"] == URL

        grok_text = grok_toml.read_text(encoding="utf-8")
        _assert_hipcortex_and_url(grok_text)
        assert "[mcp_servers.hipcortex]" in grok_text

        # ── second install → unchanged ─────────────────────────────────
        second = {
            "antigravity": _install_antigravity(URL),
            "hermes": _install_hermes(URL),
            "openclaw": _install_openclaw(URL),
            "grok": _install_grok(URL),
        }
        assert all(r == INSTALL_UNCHANGED for r in second.values()), second

        # ── uninstall each ─────────────────────────────────────────────
        assert _uninstall_antigravity() is True
        assert _uninstall_hermes() is True
        assert _uninstall_openclaw() is True
        assert _uninstall_grok() is True

        # antigravity: no hipcortex key
        if antigravity_mcp.exists():
            ag2 = json.loads(antigravity_mcp.read_text(encoding="utf-8"))
            assert "hipcortex" not in ag2.get("mcpServers", {})

        # hermes: no hipcortex block
        hermes2 = hermes_yaml.read_text(encoding="utf-8")
        assert "hipcortex:" not in hermes2

        # openclaw: no hipcortex under mcp.servers
        if openclaw_json.exists():
            oc2 = json.loads(openclaw_json.read_text(encoding="utf-8"))
            assert "hipcortex" not in oc2.get("mcp", {}).get("servers", {})

        # grok: no mcp_servers.hipcortex table
        if grok_toml.exists():
            grok2 = grok_toml.read_text(encoding="utf-8")
            assert "[mcp_servers.hipcortex]" not in grok2
            assert "hipcortex" not in grok2 or "HIPCORTEX_URL" not in grok2


@pytest.mark.host_e2e
def test_hosts_skip_when_root_missing(tmp_path, monkeypatch):
    """Without host dirs (and without env overrides), installers skip."""
    home = tmp_path / "empty_home"
    home.mkdir()
    monkeypatch.delenv("OPENCLAW_CONFIG_PATH", raising=False)
    monkeypatch.delenv("GROK_CONFIG_PATH", raising=False)

    from hipcortex.cli import (
        INSTALL_SKIPPED,
        _install_hermes,
        _install_openclaw,
        _install_grok,
    )

    with patch("hipcortex.install_hosts.Path.home", return_value=home), patch("hipcortex.cli.Path.home", return_value=home):
        # antigravity creates parent dirs (no skip) — hermes/openclaw/grok skip
        assert _install_hermes(URL) == INSTALL_SKIPPED
        assert _install_openclaw(URL) == INSTALL_SKIPPED
        assert _install_grok(URL) == INSTALL_SKIPPED
