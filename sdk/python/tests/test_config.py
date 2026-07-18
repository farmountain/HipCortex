"""Tests for hipcortex.config — load/save/resolve .hipcortex config.toml.

Run: pytest sdk/python/tests/test_config.py -q
"""

from __future__ import annotations

from pathlib import Path

import pytest

from hipcortex import config as cfg
from hipcortex.config import (
    DEFAULT_MODE,
    DEFAULT_SERVER_VERSION_POLICY,
    DEFAULT_URL,
    HipCortexSettings,
    ensure_project_config,
    get_default_actor,
    load_settings,
    load_toml_file,
    project_config_path,
    resolve_actor,
    settings_to_toml,
    write_project_config,
    write_user_config,
)


# ── helpers ──────────────────────────────────────────────────────────────────


@pytest.fixture
def user_cfg(tmp_path, monkeypatch):
    """Point USER_CONFIG_PATH at tmp so tests never touch real home."""
    path = tmp_path / "home" / ".hipcortex" / "user.toml"
    path.parent.mkdir(parents=True)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", path)
    return path


# ── load_toml_file ───────────────────────────────────────────────────────────


def test_load_toml_file_missing(tmp_path):
    assert load_toml_file(tmp_path / "nope.toml") == {}


def test_load_toml_file_empty_file(tmp_path):
    p = tmp_path / "empty.toml"
    p.write_text("", encoding="utf-8")
    assert load_toml_file(p) == {}


def test_load_toml_file_basic(tmp_path):
    p = tmp_path / "c.toml"
    p.write_text(
        'url = "http://x:1"\nactor = "a1"\nchannels = ["cli", "mcp"]\n',
        encoding="utf-8",
    )
    data = load_toml_file(p)
    assert data["url"] == "http://x:1"
    assert data["actor"] == "a1"
    assert data["channels"] == ["cli", "mcp"]


def test_load_toml_file_aliases_table(tmp_path):
    p = tmp_path / "c.toml"
    p.write_text(
        'url = "http://127.0.0.1:3030"\n\n[aliases]\nvscode-user = "proj-x"\n',
        encoding="utf-8",
    )
    data = load_toml_file(p)
    assert data["aliases"]["vscode-user"] == "proj-x"


# ── settings_to_toml / write roundtrip ───────────────────────────────────────


def test_settings_to_toml_deterministic():
    s = HipCortexSettings(
        url="http://127.0.0.1:3030",
        actor="proj-myapp",
        mode="proactive",
        channels=["claude-code", "vscode-extension"],
        data_dir="",
        server_version_policy="major_minor",
        aliases={"session_id": "proj-myapp", "vscode-user": "proj-myapp"},
    )
    # empty data_dir still written when explicitly set (not None)
    s.data_dir = "/tmp/hc"
    text = settings_to_toml(s)
    assert 'url = "http://127.0.0.1:3030"' in text
    assert 'actor = "proj-myapp"' in text
    assert 'mode = "proactive"' in text
    assert "channels = [" in text
    assert 'data_dir = "/tmp/hc"' in text
    assert 'server_version_policy = "major_minor"' in text
    assert "[aliases]" in text
    # aliases sorted
    assert text.index("session_id") < text.index("vscode-user")


def test_write_project_config_roundtrip(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    s = HipCortexSettings(
        url="http://proj:3030",
        actor="proj-actor",
        mode="proactive",
        channels=["mcp"],
        server_version_policy="exact",
        aliases={"vs": "proj-actor"},
    )
    path = write_project_config(tmp_path, s)
    assert path == project_config_path(tmp_path)
    assert path.is_file()

    loaded = load_settings(tmp_path)
    assert loaded.url == "http://proj:3030"
    assert loaded.actor == "proj-actor"
    assert loaded.mode == "proactive"
    assert loaded.channels == ["mcp"]
    assert loaded.server_version_policy == "exact"
    assert loaded.aliases["vs"] == "proj-actor"


def test_write_user_config_roundtrip(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    s = HipCortexSettings(url="http://user:1", actor="user-actor", mode="conservative")
    path = write_user_config(s)
    assert path == user_cfg
    assert path.is_file()

    # no project config → user values win over defaults
    loaded = load_settings(tmp_path)
    assert loaded.url == "http://user:1"
    assert loaded.actor == "user-actor"
    assert loaded.mode == "conservative"


# ── load_settings precedence ─────────────────────────────────────────────────


def test_load_settings_defaults(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    s = load_settings(tmp_path)
    assert s.url == DEFAULT_URL
    assert s.actor is None
    assert s.mode == DEFAULT_MODE
    assert s.channels == []
    assert s.data_dir is None
    assert s.server_version_policy == DEFAULT_SERVER_VERSION_POLICY
    assert s.aliases == {}


def test_load_settings_project_over_user(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    write_user_config(
        HipCortexSettings(url="http://user:1", actor="user-a", mode="conservative")
    )
    write_project_config(
        tmp_path,
        HipCortexSettings(url="http://proj:2", actor="proj-a", mode="proactive"),
    )

    s = load_settings(tmp_path)
    assert s.url == "http://proj:2"
    assert s.actor == "proj-a"
    assert s.mode == "proactive"


def test_load_settings_env_over_project(tmp_path, user_cfg, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_URL", "http://env:9/")
    monkeypatch.setenv("HIPCORTEX_ACTOR", "env-actor")

    write_user_config(HipCortexSettings(url="http://user:1", actor="user-a"))
    write_project_config(
        tmp_path, HipCortexSettings(url="http://proj:2", actor="proj-a")
    )

    s = load_settings(tmp_path)
    assert s.url == "http://env:9"
    assert s.actor == "env-actor"


def test_load_settings_user_only_url(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    write_user_config(HipCortexSettings(url="http://user-only:7"))
    s = load_settings(tmp_path)
    assert s.url == "http://user-only:7"
    assert s.actor is None


def test_load_settings_partial_project_keeps_user_actor(tmp_path, user_cfg, monkeypatch):
    """Project without actor key keeps user actor; url still project."""
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    write_user_config(HipCortexSettings(url="http://user:1", actor="user-a"))
    # write raw project with only url
    p = project_config_path(tmp_path)
    p.parent.mkdir(parents=True)
    p.write_text('url = "http://proj:2"\nmode = "proactive"\n', encoding="utf-8")

    s = load_settings(tmp_path)
    assert s.url == "http://proj:2"
    assert s.actor == "user-a"
    assert s.mode == "proactive"


def test_load_settings_empty_data_dir_becomes_none(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    p = project_config_path(tmp_path)
    p.parent.mkdir(parents=True)
    p.write_text(
        'url = "http://127.0.0.1:3030"\ndata_dir = ""\n',
        encoding="utf-8",
    )
    s = load_settings(tmp_path)
    assert s.data_dir is None


def test_load_settings_merges_aliases(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    write_user_config(
        HipCortexSettings(aliases={"from-user": "u", "shared": "user-wins-before-proj"})
    )
    write_project_config(
        tmp_path,
        HipCortexSettings(aliases={"from-proj": "p", "shared": "proj-wins"}),
    )
    s = load_settings(tmp_path)
    assert s.aliases["from-user"] == "u"
    assert s.aliases["from-proj"] == "p"
    assert s.aliases["shared"] == "proj-wins"


# ── resolve_actor ────────────────────────────────────────────────────────────


def test_resolve_actor_alias():
    s = HipCortexSettings(aliases={"vscode-user": "proj-myapp", "session_id": "proj-myapp"})
    assert resolve_actor("vscode-user", s) == "proj-myapp"
    assert resolve_actor("session_id", s) == "proj-myapp"
    assert resolve_actor("other", s) == "other"
    assert resolve_actor("", s) == ""


def test_get_default_actor(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    assert get_default_actor(tmp_path) is None
    ensure_project_config(tmp_path, url="http://x", actor="proj-a")
    assert get_default_actor(tmp_path) == "proj-a"
    monkeypatch.setenv("HIPCORTEX_ACTOR", "env-a")
    assert get_default_actor(tmp_path) == "env-a"


# ── ensure_project_config ────────────────────────────────────────────────────


def test_ensure_project_config_creates(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    path = ensure_project_config(
        tmp_path,
        url="http://127.0.0.1:3030",
        actor="proj-myapp",
        mode="proactive",
        channels=["claude-code", "vscode-extension"],
    )
    assert path == project_config_path(tmp_path)
    assert path.is_file()

    s = load_settings(tmp_path)
    assert s.url == "http://127.0.0.1:3030"
    assert s.actor == "proj-myapp"
    assert s.mode == "proactive"
    assert s.channels == ["claude-code", "vscode-extension"]


def test_ensure_project_config_merges_preserves_aliases(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    write_project_config(
        tmp_path,
        HipCortexSettings(
            url="http://old:1",
            actor="old-actor",
            aliases={"keep": "me"},
            channels=["old-ch"],
        ),
    )
    ensure_project_config(
        tmp_path,
        url="http://new:2",
        actor="new-actor",
        mode="conservative",
        channels=None,  # keep prior channels
    )
    s = load_settings(tmp_path)
    assert s.url == "http://new:2"
    assert s.actor == "new-actor"
    assert s.channels == ["old-ch"]
    assert s.aliases["keep"] == "me"


def test_ensure_project_config_replaces_channels(tmp_path, user_cfg, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.delenv("HIPCORTEX_ACTOR", raising=False)

    ensure_project_config(tmp_path, url="http://a", channels=["a"])
    ensure_project_config(tmp_path, url="http://a", channels=["b", "c"])
    s = load_settings(tmp_path)
    assert s.channels == ["b", "c"]


# ── to_dict ──────────────────────────────────────────────────────────────────


def test_to_dict():
    s = HipCortexSettings(url="http://x", actor="a", channels=["c"])
    d = s.to_dict()
    assert d["url"] == "http://x"
    assert d["actor"] == "a"
    assert d["channels"] == ["c"]
    assert d["aliases"] == {}
