"""Tests: client/async client resolve base_url via load_settings.

Run: pytest sdk/python/tests/test_client_config.py -q
"""

from __future__ import annotations

from hipcortex import config as cfg
from hipcortex.config import ensure_project_config


def test_hipcortex_client_uses_settings_url(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://from-config:3030")

    from hipcortex.client import HipCortexClient

    client = HipCortexClient()
    assert client.base_url == "http://from-config:3030"


def test_hipcortex_client_explicit_url_wins(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_URL", "http://env:1")
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://from-config:3030")

    from hipcortex.client import HipCortexClient

    client = HipCortexClient(base_url="http://explicit:2/")
    assert client.base_url == "http://explicit:2"


def test_hipcortex_client_env_over_project(tmp_path, monkeypatch):
    monkeypatch.setenv("HIPCORTEX_URL", "http://env:9/")
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://from-config:3030")

    from hipcortex.client import HipCortexClient

    client = HipCortexClient()
    assert client.base_url == "http://env:9"


def test_async_client_uses_settings_url(tmp_path, monkeypatch):
    monkeypatch.delenv("HIPCORTEX_URL", raising=False)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cfg, "USER_CONFIG_PATH", tmp_path / "user.toml")
    ensure_project_config(tmp_path, url="http://async-cfg:4040")

    from hipcortex.async_client import AsyncHipCortexClient

    client = AsyncHipCortexClient()
    assert client.base_url == "http://async-cfg:4040"
