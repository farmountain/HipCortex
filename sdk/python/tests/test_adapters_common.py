"""Shared adapter factories + from_settings (no network).

Run: pytest sdk/python/tests/test_adapters_common.py -q
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from hipcortex import config as cfg
from hipcortex.config import ensure_project_config


def test_client_from_settings_uses_config_url(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://adapter-cfg:3030", actor="proj-actor")

    from hipcortex.adapters.common import client_from_settings

    client = client_from_settings()
    assert client.base_url == "http://adapter-cfg:3030"


def test_default_session_id_falls_back_to_default(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://x:1")

    from hipcortex.adapters.common import default_session_id

    assert default_session_id() == "default"


def test_default_session_id_from_project(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://x:1", actor="crew-a")

    from hipcortex.adapters.common import default_session_id

    assert default_session_id() == "crew-a"


def test_bootstrap_live_beliefs_formats_summary():
    from hipcortex.adapters.common import bootstrap_live_beliefs

    mock_client = MagicMock()
    mock_client.live_beliefs.return_value = {
        "summary": "Live beliefs: 2",
        "symbolic_facts": {"nodes": [{"label": "postgres"}]},
        "hypotheses": [{"hypothesis": "use petgraph", "confidence": 0.9}],
    }
    text = bootstrap_live_beliefs(mock_client, actor="a1", limit=5)
    assert "live_beliefs" in text
    assert "actor=a1" in text
    assert "Live beliefs: 2" in text
    assert "postgres" in text
    mock_client.live_beliefs.assert_called_once()


def test_bootstrap_live_beliefs_empty_on_error():
    from hipcortex.adapters.common import bootstrap_live_beliefs

    mock_client = MagicMock()
    mock_client.live_beliefs.side_effect = RuntimeError("down")
    assert bootstrap_live_beliefs(mock_client) == ""


def test_hipcortex_memory_from_settings(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://lc:3030", actor="lc-actor")

    from hipcortex.langchain_memory import HipCortexMemory

    mem = HipCortexMemory.from_settings()
    assert mem.session_id == "lc-actor"
    assert mem.client.base_url == "http://lc:3030"
    assert mem.use_live_beliefs is False


def test_hipcortex_memory_from_settings_explicit_session(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://lc:3030", actor="ignored")

    from hipcortex.langchain_memory import HipCortexMemory

    mem = HipCortexMemory.from_settings(session_id="override", use_live_beliefs=True)
    assert mem.session_id == "override"
    assert mem.use_live_beliefs is True


def test_hipcortex_memory_live_beliefs_prepend_once():
    from hipcortex.langchain_memory import HipCortexMemory

    mock_client = MagicMock()
    mock_client.get_conversation_history.return_value = [
        {
            "action": "human_message",
            "target": "hi",
            "timestamp": "2026-01-01T00:00:00Z",
        },
    ]
    mock_client.live_beliefs.return_value = {"summary": "substrate ready"}

    mem = HipCortexMemory(
        client=mock_client, session_id="s1", use_live_beliefs=True
    )
    first = mem.load_memory_variables({})
    second = mem.load_memory_variables({})

    assert "substrate ready" in first["history"]
    assert "Human: hi" in first["history"]
    # once only
    assert mock_client.live_beliefs.call_count == 1
    assert "substrate ready" not in second["history"]
    assert "Human: hi" in second["history"]


def test_make_memory_tools_defaults(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://crew:3030", actor="crew-agent")

    from hipcortex.adapters.crewai import make_memory_tools

    tools = make_memory_tools()
    assert len(tools) == 3
    assert tools[0]._agent_id == "crew-agent"
    assert tools[0]._client.base_url == "http://crew:3030"
    assert tools[1]._agent_id == "crew-agent"
    assert tools[2]._agent_id == "crew-agent"


def test_make_memory_tools_explicit_client():
    from hipcortex.adapters.crewai import make_memory_tools

    mock_client = MagicMock()
    tools = make_memory_tools(client=mock_client, agent_id="explicit")
    assert len(tools) == 3
    assert tools[0]._client is mock_client
    assert tools[0]._agent_id == "explicit"


def test_autogen_from_settings(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://ag:3030", actor="ag-agent")

    from hipcortex.adapters.autogen import HipCortexAutoGenMemory

    mem = HipCortexAutoGenMemory.from_settings()
    assert mem._agent_id == "ag-agent"
    assert mem._client.base_url == "http://ag:3030"


def test_adapters_init_exports():
    from hipcortex.adapters import (
        bootstrap_live_beliefs,
        client_from_settings,
        default_session_id,
    )

    assert callable(client_from_settings)
    assert callable(default_session_id)
    assert callable(bootstrap_live_beliefs)
