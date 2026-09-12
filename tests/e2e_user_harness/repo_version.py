"""Single source of truth for the product version asserted by the E2E harness.

The harness previously hard-coded the version string in each phase file, which
meant a version bump required editing N test literals and a missed edit produced
a *false green* rather than a failure. Reading the repo-root ``VERSION`` file
instead makes every assertion below a genuine alignment check: it fails when any
published surface disagrees with the declared version.

``VERSION`` is the repo's single source of truth (see ``CLAUDE.md`` and
``scripts/stamp_versions.py``).
"""

from __future__ import annotations

from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
_VERSION_FILE = _REPO_ROOT / "VERSION"


def repo_version() -> str:
    """Return the product version declared by the repo-root ``VERSION`` file."""
    text = _VERSION_FILE.read_text(encoding="utf-8")
    return text.strip().splitlines()[0].split("#", 1)[0].strip()


def repo_root() -> Path:
    """Return the repository root directory."""
    return _REPO_ROOT
