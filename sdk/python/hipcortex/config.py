"""HipCortex project/user config — `.hipcortex/config.toml` + `~/.hipcortex/user.toml`.

Resolution (url):  HIPCORTEX_URL > project config > user config > default
Resolution (actor): HIPCORTEX_ACTOR > project > user > None

No third-party TOML deps: tomllib (3.11+) read; deterministic writer for known keys.
"""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Union

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover — package requires-python is >=3.9; config needs 3.11+ or tomli
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ImportError as _exc:
        raise ImportError(
            "hipcortex.config requires Python 3.11+ (tomllib) or the 'tomli' package"
        ) from _exc

PROJECT_DIR_NAME = ".hipcortex"
PROJECT_CONFIG_NAME = "config.toml"
USER_CONFIG_PATH: Path = Path.home() / ".hipcortex" / "user.toml"

DEFAULT_URL = "http://127.0.0.1:3030"
DEFAULT_MODE = "conservative"
DEFAULT_SERVER_VERSION_POLICY = "major_minor"

ENV_URL = "HIPCORTEX_URL"
ENV_ACTOR = "HIPCORTEX_ACTOR"

PathLike = Union[str, Path]


@dataclass
class HipCortexSettings:
    """Resolved HipCortex settings (project + user + env)."""

    url: str = DEFAULT_URL
    actor: Optional[str] = None
    mode: str = DEFAULT_MODE  # proactive | conservative
    channels: List[str] = field(default_factory=list)
    data_dir: Optional[str] = None
    server_version_policy: str = DEFAULT_SERVER_VERSION_POLICY  # major_minor | exact | any_healthy
    aliases: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "url": self.url,
            "actor": self.actor,
            "mode": self.mode,
            "channels": list(self.channels),
            "data_dir": self.data_dir,
            "server_version_policy": self.server_version_policy,
            "aliases": dict(self.aliases),
        }


def project_config_path(project_root: PathLike) -> Path:
    """Return `<root>/.hipcortex/config.toml`."""
    return Path(project_root) / PROJECT_DIR_NAME / PROJECT_CONFIG_NAME


def load_toml_file(path: PathLike) -> Dict[str, Any]:
    """Load a TOML file as a dict. Missing/unreadable → empty dict."""
    p = Path(path)
    if not p.is_file():
        return {}
    try:
        with p.open("rb") as fh:
            data = tomllib.load(fh)
    except (OSError, ValueError, TypeError):
        return {}
    return data if isinstance(data, dict) else {}


def _escape_toml_str(value: str) -> str:
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def _format_toml_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int) and not isinstance(value, bool):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        return f'"{_escape_toml_str(value)}"'
    if isinstance(value, list):
        inner = ", ".join(_format_toml_value(v) for v in value)
        return f"[{inner}]"
    raise TypeError(f"unsupported TOML value type: {type(value)!r}")


def settings_to_toml(settings: HipCortexSettings) -> str:
    """Deterministic TOML for HipCortexSettings keys only."""
    lines: List[str] = [
        f"url = {_format_toml_value(settings.url)}",
    ]
    if settings.actor is not None:
        lines.append(f"actor = {_format_toml_value(settings.actor)}")
    lines.append(f"mode = {_format_toml_value(settings.mode)}")
    lines.append(f"channels = {_format_toml_value(list(settings.channels))}")
    if settings.data_dir is not None:
        lines.append(f"data_dir = {_format_toml_value(settings.data_dir)}")
    lines.append(
        f"server_version_policy = {_format_toml_value(settings.server_version_policy)}"
    )
    if settings.aliases:
        lines.append("")
        lines.append("[aliases]")
        for key in sorted(settings.aliases):
            lines.append(f"{key} = {_format_toml_value(settings.aliases[key])}")
    lines.append("")
    return "\n".join(lines)


def _normalize_data_dir(value: Any) -> Optional[str]:
    if value is None:
        return None
    text = str(value).strip()
    return text if text else None


def _layer_from_toml(data: Mapping[str, Any]) -> Dict[str, Any]:
    """Extract known HipCortexSettings fields from a raw TOML mapping."""
    layer: Dict[str, Any] = {}
    if "url" in data and data["url"] is not None:
        layer["url"] = str(data["url"]).rstrip("/")
    if "actor" in data:
        actor = data["actor"]
        layer["actor"] = None if actor is None or str(actor).strip() == "" else str(actor)
    if "mode" in data and data["mode"] is not None:
        layer["mode"] = str(data["mode"])
    if "channels" in data and data["channels"] is not None:
        raw = data["channels"]
        if isinstance(raw, list):
            layer["channels"] = [str(c) for c in raw]
        elif isinstance(raw, str) and raw.strip():
            layer["channels"] = [raw.strip()]
    if "data_dir" in data:
        layer["data_dir"] = _normalize_data_dir(data["data_dir"])
    if "server_version_policy" in data and data["server_version_policy"] is not None:
        layer["server_version_policy"] = str(data["server_version_policy"])
    aliases = data.get("aliases")
    if isinstance(aliases, dict):
        layer["aliases"] = {str(k): str(v) for k, v in aliases.items()}
    return layer


def _apply_layer(settings: HipCortexSettings, data: Mapping[str, Any]) -> HipCortexSettings:
    """Return new settings with non-empty layer keys overlaid."""
    layer = _layer_from_toml(data)
    if not layer:
        return HipCortexSettings(
            url=settings.url,
            actor=settings.actor,
            mode=settings.mode,
            channels=list(settings.channels),
            data_dir=settings.data_dir,
            server_version_policy=settings.server_version_policy,
            aliases=dict(settings.aliases),
        )
    aliases = dict(settings.aliases)
    if "aliases" in layer:
        aliases.update(layer["aliases"])
    return HipCortexSettings(
        url=layer.get("url", settings.url),
        actor=layer["actor"] if "actor" in layer else settings.actor,
        mode=layer.get("mode", settings.mode),
        channels=list(layer["channels"]) if "channels" in layer else list(settings.channels),
        data_dir=layer["data_dir"] if "data_dir" in layer else settings.data_dir,
        server_version_policy=layer.get(
            "server_version_policy", settings.server_version_policy
        ),
        aliases=aliases,
    )


def write_project_config(path_or_cwd: PathLike, settings: HipCortexSettings) -> Path:
    """Write project config under `<path_or_cwd>/.hipcortex/config.toml`. Return path."""
    root = Path(path_or_cwd)
    cfg_path = project_config_path(root)
    cfg_path.parent.mkdir(parents=True, exist_ok=True)
    cfg_path.write_text(settings_to_toml(settings), encoding="utf-8")
    return cfg_path


def write_user_config(
    settings: HipCortexSettings,
    path: Optional[PathLike] = None,
) -> Path:
    """Write user config (default: `~/.hipcortex/user.toml`). Return path."""
    cfg_path = Path(path) if path is not None else USER_CONFIG_PATH
    cfg_path.parent.mkdir(parents=True, exist_ok=True)
    cfg_path.write_text(settings_to_toml(settings), encoding="utf-8")
    return cfg_path


def load_settings(project_root: Optional[PathLike] = None) -> HipCortexSettings:
    """Load merged settings: defaults < user < project < env.

    ``project_root`` defaults to cwd. Project file:
    ``<root>/.hipcortex/config.toml``. User file: ``USER_CONFIG_PATH``.
    """
    root = Path(project_root) if project_root is not None else Path.cwd()
    settings = HipCortexSettings()
    settings = _apply_layer(settings, load_toml_file(USER_CONFIG_PATH))
    settings = _apply_layer(settings, load_toml_file(project_config_path(root)))

    env_url = os.environ.get(ENV_URL)
    if env_url is not None and env_url.strip():
        settings.url = env_url.strip().rstrip("/")

    env_actor = os.environ.get(ENV_ACTOR)
    if env_actor is not None and env_actor.strip():
        settings.actor = env_actor.strip()

    return settings


def resolve_actor(name: str, settings: HipCortexSettings) -> str:
    """Map ``name`` through ``settings.aliases``; return original if unmapped."""
    if not name:
        return name
    return settings.aliases.get(name, name)


def ensure_project_config(
    cwd: PathLike,
    url: str,
    actor: Optional[str] = None,
    mode: str = DEFAULT_MODE,
    channels: Optional[List[str]] = None,
) -> Path:
    """Create/merge project `.hipcortex/config.toml`. Return path written.

    Existing keys preserved unless overridden by arguments. ``channels`` only
    replaces when a list is passed (``None`` keeps prior channels).
    """
    root = Path(cwd)
    cfg_path = project_config_path(root)
    existing = load_toml_file(cfg_path)
    merged = _apply_layer(HipCortexSettings(), existing)
    merged.url = str(url).rstrip("/")
    if actor is not None:
        merged.actor = actor if str(actor).strip() else None
    if mode:
        merged.mode = mode
    if channels is not None:
        merged.channels = list(channels)
    return write_project_config(root, merged)
