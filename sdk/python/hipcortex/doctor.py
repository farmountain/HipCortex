"""HipCortex doctor — post-install verification (health, version, optional probe).

Usage:
    from hipcortex.doctor import run_doctor, format_report
    report = run_doctor(probe=False)
    print(format_report(report))

CLI:
    hipcortex doctor
    hipcortex doctor --probe
    HIPCORTEX_DOCTOR_OFFLINE=1 hipcortex doctor   # skip network
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

import requests

from .config import DEFAULT_URL, load_settings

OFFLINE_ENV = "HIPCORTEX_DOCTOR_OFFLINE"

# Case-sensitive harness markers required in proactive SKILL.md
SKILL_HARNESS_MARKERS = ("MUST", "live_beliefs")

PathLike = Union[str, Path]


def package_skill_path() -> Path:
    """Bundled SKILL.md template shipped with the package (unit of truth)."""
    return Path(__file__).resolve().parent / "install" / "SKILL.md"


def installed_skill_path() -> Path:
    """Claude Code skill path: ~/.claude/skills/hipcortex/SKILL.md."""
    return Path.home() / ".claude" / "skills" / "hipcortex" / "SKILL.md"


def skill_missing_markers(text: str) -> List[str]:
    """Return harness markers missing from skill text (case-sensitive)."""
    return [m for m in SKILL_HARNESS_MARKERS if m not in text]


def check_skill_file(
    report: DoctorReport,
    name: str,
    path: Path,
    *,
    missing_status: str = "warn",
    incomplete_status: str = "fail",
) -> CheckResult:
    """Verify a SKILL.md path contains MUST + live_beliefs harness language.

    Args:
        report: DoctorReport to append to.
        name: Check name (e.g. skill_package, skill_installed).
        path: Filesystem path to SKILL.md.
        missing_status: Status when file absent (package: fail; installed: warn).
        incomplete_status: Status when file present but markers missing.
    """
    if not path.is_file():
        return report.add(
            name,
            missing_status,
            f"SKILL.md not found at {path}",
            detail={"path": str(path)},
        )
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        return report.add(
            name,
            "fail",
            f"cannot read SKILL.md at {path}: {e}",
            detail={"path": str(path)},
        )
    missing = skill_missing_markers(text)
    if missing:
        return report.add(
            name,
            incomplete_status,
            f"SKILL.md missing harness language: {', '.join(missing)}",
            detail={"path": str(path), "missing": missing},
        )
    return report.add(
        name,
        "ok",
        f"SKILL.md has MUST + live_beliefs ({path})",
        detail={"path": str(path)},
    )


def add_skill_checks(
    report: DoctorReport,
    *,
    package_path: Optional[PathLike] = None,
    installed_path: Optional[PathLike] = None,
) -> None:
    """Always-on file checks for package template + optional installed skill.

    Offline-safe (no network). Package template is unit of truth (fail if bad).
    Installed Claude skill: warn if missing; fail if present but incomplete.
    """
    pkg = Path(package_path) if package_path is not None else package_skill_path()
    inst = Path(installed_path) if installed_path is not None else installed_skill_path()
    check_skill_file(
        report,
        "skill_package",
        pkg,
        missing_status="fail",
        incomplete_status="fail",
    )
    check_skill_file(
        report,
        "skill_installed",
        inst,
        missing_status="warn",
        incomplete_status="fail",
    )


@dataclass
class CheckResult:
    """Single doctor check outcome."""

    name: str
    status: str  # ok | fail | warn | skip
    message: str
    detail: Optional[Dict[str, Any]] = None


@dataclass
class DoctorReport:
    """Aggregated doctor results."""

    url: str
    checks: List[CheckResult] = field(default_factory=list)
    ok: bool = True  # False if any check failed

    def add(
        self,
        name: str,
        status: str,
        message: str,
        detail: Optional[Dict[str, Any]] = None,
    ) -> CheckResult:
        result = CheckResult(name=name, status=status, message=message, detail=detail)
        self.checks.append(result)
        if status == "fail":
            self.ok = False
        return result


def resolve_url(url: Optional[str] = None) -> str:
    """Resolve base URL: explicit arg > load_settings (env > project > user > default)."""
    if url:
        return url.rstrip("/")
    return load_settings().url.rstrip("/")


def is_offline() -> bool:
    """True when HIPCORTEX_DOCTOR_OFFLINE is truthy (1/true/yes)."""
    return os.getenv(OFFLINE_ENV, "").strip().lower() in ("1", "true", "yes")


def _session_get(
    session: requests.Session,
    url: str,
    timeout: float,
) -> requests.Response:
    return session.get(url, timeout=timeout)


def _session_post(
    session: requests.Session,
    url: str,
    json: Dict[str, Any],
    timeout: float,
) -> requests.Response:
    return session.post(url, json=json, timeout=timeout)


def run_doctor(
    url: Optional[str] = None,
    probe: bool = False,
    timeout: float = 5.0,
    session: Optional[requests.Session] = None,
    *,
    package_skill: Optional[PathLike] = None,
    installed_skill: Optional[PathLike] = None,
) -> DoctorReport:
    """Run post-install checks against a HipCortex server.

    Args:
        url: Base URL (falls back to :func:`hipcortex.config.load_settings`).
        probe: If True and online, POST /memory/add + /memory/search roundtrip.
        timeout: Per-request timeout seconds.
        session: Optional requests.Session (for tests / injection).
        package_skill: Override path to package install/SKILL.md (tests).
        installed_skill: Override path to ~/.claude/.../SKILL.md (tests).

    Returns:
        DoctorReport with structured checks (ok/fail/warn/skip).

    Note:
        Proactive SKILL harness checks (MUST + live_beliefs) always run,
        including offline mode — pure filesystem, no network.
    """
    settings = load_settings()
    base = url.rstrip("/") if url else settings.url.rstrip("/")
    report = DoctorReport(url=base)

    # ── proactive SKILL harness (always; offline-safe) ───────────────────
    add_skill_checks(
        report,
        package_path=package_skill,
        installed_path=installed_skill,
    )

    if is_offline():
        report.add(
            "health",
            "skip",
            f"{OFFLINE_ENV}=1 — network checks skipped",
        )
        report.add(
            "version",
            "skip",
            "offline — version not fetched",
        )
        if probe:
            report.add(
                "probe",
                "skip",
                "offline — add/search probe skipped",
            )
        return report

    sess = session or requests.Session()

    # ── GET /health ──────────────────────────────────────────────────────
    health_url = f"{base}/health"
    try:
        resp = _session_get(sess, health_url, timeout)
    except requests.RequestException as e:
        report.add(
            "health",
            "fail",
            f"not reachable at {health_url}: {e}",
        )
        report.add("version", "skip", "health failed — version not checked")
        if probe:
            report.add("probe", "skip", "health failed — probe skipped")
        return report

    if resp.status_code != 200:
        report.add(
            "health",
            "fail",
            f"HTTP {resp.status_code} from {health_url}",
            detail={"status_code": resp.status_code},
        )
        report.add("version", "skip", "health not ok — version not checked")
        if probe:
            report.add("probe", "skip", "health not ok — probe skipped")
        return report

    body: Dict[str, Any] = {}
    try:
        body = resp.json() if resp.content else {}
    except ValueError:
        body = {}

    report.add(
        "health",
        "ok",
        f"reachable at {base}",
        detail=body if isinstance(body, dict) else None,
    )

    # ── version ──────────────────────────────────────────────────────────
    version = None
    if isinstance(body, dict):
        version = body.get("version")
    if version:
        report.add(
            "version",
            "ok",
            f"server version {version}",
            detail={"version": version},
        )
    else:
        report.add(
            "version",
            "warn",
            "health ok but version field missing",
            detail=body if isinstance(body, dict) else None,
        )

    # ── optional probe: add + search ─────────────────────────────────────
    if not probe:
        return report

    actor = settings.actor or "hipcortex-doctor"
    action = "probe"
    target = "doctor-roundtrip"
    try:
        add_resp = _session_post(
            sess,
            f"{base}/memory/add",
            {
                "actor": actor,
                "action": action,
                "target": target,
                "record_type": "Temporal",
                "metadata": {"source": "hipcortex-doctor"},
            },
            timeout,
        )
        add_resp.raise_for_status()
        add_body = add_resp.json() if add_resp.content else {}
    except (requests.RequestException, ValueError) as e:
        report.add("probe", "fail", f"POST /memory/add failed: {e}")
        return report

    try:
        search_resp = _session_post(
            sess,
            f"{base}/memory/search",
            {"query": target, "limit": 5},
            timeout,
        )
        search_resp.raise_for_status()
        search_body = search_resp.json() if search_resp.content else {}
    except (requests.RequestException, ValueError) as e:
        report.add(
            "probe",
            "fail",
            f"POST /memory/search failed after add: {e}",
            detail={"add": add_body if isinstance(add_body, dict) else None},
        )
        return report

    results = search_body.get("results", []) if isinstance(search_body, dict) else []
    report.add(
        "probe",
        "ok",
        f"add+search ok ({len(results)} result(s))",
        detail={
            "add": add_body if isinstance(add_body, dict) else None,
            "search_count": len(results),
        },
    )
    return report


def format_report(report: DoctorReport) -> str:
    """Human-readable multi-line doctor report."""
    icons = {"ok": "✓", "fail": "✗", "warn": "!", "skip": "–"}
    lines = [f"HipCortex doctor — {report.url}"]
    for c in report.checks:
        icon = icons.get(c.status, "?")
        lines.append(f"  {icon} [{c.status}] {c.name}: {c.message}")
    if report.ok:
        if any(c.status == "warn" for c in report.checks):
            lines.append("Result: PASS (with warnings)")
        elif all(c.status == "skip" for c in report.checks):
            lines.append("Result: SKIP (offline)")
        else:
            lines.append("Result: PASS")
    else:
        lines.append("Result: FAIL")
        lines.append("  Hint: hipcortex start   # or set HIPCORTEX_URL")
    return "\n".join(lines)


def doctor_exit_code(report: DoctorReport) -> int:
    """0 if no failures (warn/skip ok); 1 if any fail."""
    return 0 if report.ok else 1
