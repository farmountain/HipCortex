"""Framework adapters for HipCortex — shared factories, CrewAI, AutoGen."""

from .common import (
    bootstrap_live_beliefs,
    client_from_settings,
    default_session_id,
)

__all__ = [
    "bootstrap_live_beliefs",
    "client_from_settings",
    "default_session_id",
]
