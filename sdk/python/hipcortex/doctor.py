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
from typing import Any, Dict, List, Optional

import requests

DEFAULT_URL = "http://127.0.0.1:3030"
OFFLINE_ENV = "HIPCORTEX_DOCTOR_OFFLINE"


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
    """Resolve base URL from arg, HIPCORTEX_URL, or default localhost:3030."""
    if url:
        return url.rstrip("/")
    return os.getenv("HIPCORTEX_URL", DEFAULT_URL).rstrip("/")


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
) -> DoctorReport:
    """Run post-install checks against a HipCortex server.

    Args:
        url: Base URL (falls back to HIPCORTEX_URL / http://127.0.0.1:3030).
        probe: If True and online, POST /memory/add + /memory/search roundtrip.
        timeout: Per-request timeout seconds.
        session: Optional requests.Session (for tests / injection).

    Returns:
        DoctorReport with structured checks (ok/fail/warn/skip).
    """
    base = resolve_url(url)
    report = DoctorReport(url=base)

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

    actor = "hipcortex-doctor"
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
