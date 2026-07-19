"""HipCortex doctor — post-install verification (health, version, optional probe).

Usage:
    from hipcortex.doctor import run_doctor, format_report
    report = run_doctor(probe=False)
    print(format_report(report))

CLI:
    hipcortex doctor
    hipcortex doctor --probe
    HIPCORTEX_DOCTOR_OFFLINE=1 hipcortex doctor   # skip network

Probe (``--probe``) is local-only by default: only localhost / 127.0.0.1 / ::1 /
0.0.0.0 may run add+search. Remote URLs skip probe unless
``HIPCORTEX_DOCTOR_PROBE_REMOTE=1``. Success requires a search hit matching the
unique probe target (empty results fail). After a successful roundtrip the probe
best-effort DELETEs the created memory record when an id is returned.
"""

from __future__ import annotations

import os
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple, Union
from urllib.parse import urlparse

import requests

from .config import DEFAULT_SERVER_VERSION_POLICY, DEFAULT_URL, load_settings

OFFLINE_ENV = "HIPCORTEX_DOCTOR_OFFLINE"
PROBE_REMOTE_ENV = "HIPCORTEX_DOCTOR_PROBE_REMOTE"
_LOCAL_HOSTS = frozenset({"localhost", "127.0.0.1", "::1", "0.0.0.0"})

# Case-sensitive harness markers required in proactive SKILL.md
SKILL_HARNESS_MARKERS = ("MUST", "live_beliefs")

PathLike = Union[str, Path]


def _parse_major_minor(version: str) -> Optional[Tuple[int, int]]:
    """Parse leading X.Y from a version string; ignore patch/pre-release."""
    cleaned = str(version).strip().lstrip("vV")
    parts = cleaned.split(".")
    if len(parts) < 2:
        return None
    try:
        return int(parts[0]), int(parts[1])
    except ValueError:
        return None


def version_policy_ok(
    server_version: Optional[str],
    client_version: str,
    policy: str = "major_minor",
) -> Tuple[bool, str]:
    """Return (ok, message) for client/server version policy.

    Policies:
    - ``any_healthy``: ok whenever health already succeeded (version optional)
    - ``exact``: server string equals client string
    - ``major_minor`` (default): major+minor match; patch may differ
    - unknown policy treated as ``major_minor``

    Missing server version under major_minor/exact is not a hard fail —
    ``ok=False`` so the doctor can **warn**; it does not fail the report alone.
    """
    pol = (policy or DEFAULT_SERVER_VERSION_POLICY).strip().lower()
    if pol not in ("major_minor", "exact", "any_healthy"):
        pol = "major_minor"

    if pol == "any_healthy":
        return True, f"any_healthy policy (client {client_version})"

    if not server_version:
        return (
            False,
            f"server version missing (client {client_version}, policy {pol})",
        )

    sv = str(server_version)
    cv = str(client_version)

    if pol == "exact":
        if sv == cv:
            return True, f"exact match {sv}"
        return False, f"exact mismatch: server {sv} vs client {cv}"

    # major_minor
    s = _parse_major_minor(sv)
    c = _parse_major_minor(cv)
    if s is None or c is None:
        return (
            False,
            f"unparseable version server={sv!r} client={cv!r}",
        )
    if s == c:
        return (
            True,
            f"major_minor match {s[0]}.{s[1]} (server {sv}, client {cv})",
        )
    return False, f"major_minor mismatch: server {sv} vs client {cv}"


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


def is_probe_remote_allowed() -> bool:
    """True when HIPCORTEX_DOCTOR_PROBE_REMOTE is truthy (1/true/yes)."""
    return os.getenv(PROBE_REMOTE_ENV, "").strip().lower() in ("1", "true", "yes")


def is_local_doctor_url(url: str) -> bool:
    """True when URL host is localhost-like (safe for default probe).

    Accepts host in {localhost, 127.0.0.1, ::1, 0.0.0.0}. Non-local hosts
    (e.g. example.com, fly.dev) return False so probe is gated.
    """
    if not url or not str(url).strip():
        return False
    raw = str(url).strip()
    if "://" not in raw:
        raw = f"http://{raw}"
    try:
        parsed = urlparse(raw)
    except ValueError:
        return False
    host = (parsed.hostname or "").strip().lower()
    return host in _LOCAL_HOSTS


def _result_contains_target(item: Any, target: str) -> bool:
    """True if item (or nested structure) mentions the probe target."""
    if item is None:
        return False
    if isinstance(item, str):
        return target in item
    if isinstance(item, dict):
        t = item.get("target")
        if t is not None and target in str(t):
            return True
        rec = item.get("record")
        if rec is not None and _result_contains_target(rec, target):
            return True
        # string form of whole item (unique uuid target → low false-positive)
        return target in str(item)
    if isinstance(item, (list, tuple)):
        return any(_result_contains_target(x, target) for x in item)
    return target in str(item)


def probe_result_matches_target(results: Any, target: str) -> bool:
    """True if at least one search result matches the probe target."""
    if not results or not isinstance(results, list):
        return False
    return any(_result_contains_target(item, target) for item in results)


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


def _session_delete(
    session: requests.Session,
    url: str,
    timeout: float,
) -> requests.Response:
    return session.delete(url, timeout=timeout)


def _extract_probe_record_id(add_body: Any) -> Optional[str]:
    """Pull record id from add response (``record_id`` or ``id``)."""
    if not isinstance(add_body, dict):
        return None
    rid = add_body.get("record_id")
    if rid is None:
        rid = add_body.get("id")
    if rid is None:
        return None
    text = str(rid).strip()
    return text or None


def _probe_cleanup(
    session: requests.Session,
    base: str,
    record_id: Optional[str],
    timeout: float,
) -> str:
    """Best-effort DELETE of probe record. Return ok | skipped_no_id | failed:..."""
    if not record_id:
        return "skipped_no_id"
    try:
        resp = _session_delete(session, f"{base}/memory/{record_id}", timeout)
        resp.raise_for_status()
        return "ok"
    except Exception as e:  # noqa: BLE001 — probe must stay green on cleanup fail
        return f"failed: {e}"


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
            Local URLs only by default; set HIPCORTEX_DOCTOR_PROBE_REMOTE=1 for remote.
            Fails unless search returns a hit matching the unique probe target.
            On success (and on search fail/miss after add), best-effort
            DELETE /memory/{id} (detail ``cleanup`` status) to avoid orphan records.
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

    # ── version (policy from settings; never hard-fails alone) ───────────
    from . import __version__ as client_version

    version = None
    if isinstance(body, dict):
        version = body.get("version")
    policy = settings.server_version_policy or DEFAULT_SERVER_VERSION_POLICY
    ok_ver, ver_msg = version_policy_ok(version, client_version, policy)
    detail: Dict[str, Any] = {
        "server_version": version,
        "client_version": client_version,
        "policy": policy,
    }
    if ok_ver:
        report.add("version", "ok", ver_msg, detail=detail)
    else:
        # mismatch or missing → warn (not fail)
        report.add("version", "warn", ver_msg, detail=detail)

    # ── optional probe: add + search ─────────────────────────────────────
    if not probe:
        return report

    # Remote URL gate: health/version already ran; probe only on local by default
    remote_probe = False
    if not is_local_doctor_url(base):
        if not is_probe_remote_allowed():
            report.add(
                "probe",
                "skip",
                f"remote URL blocked for probe (set {PROBE_REMOTE_ENV}=1 to allow)",
            )
            return report
        remote_probe = True

    actor = settings.actor or "hipcortex-doctor"
    action = "probe"
    target = f"doctor-roundtrip-{uuid.uuid4().hex[:12]}"
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

    record_id = _extract_probe_record_id(add_body)

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
        cleanup_status = _probe_cleanup(sess, base, record_id, timeout)
        report.add(
            "probe",
            "fail",
            f"POST /memory/search failed after add: {e}",
            detail={
                "add": add_body if isinstance(add_body, dict) else None,
                "cleanup": cleanup_status,
                "record_id": record_id,
            },
        )
        return report

    results = search_body.get("results", []) if isinstance(search_body, dict) else []
    if not results or not probe_result_matches_target(results, target):
        n = len(results) if isinstance(results, list) else 0
        cleanup_status = _probe_cleanup(sess, base, record_id, timeout)
        report.add(
            "probe",
            "fail",
            f"add ok but search missed probe target ({n} result(s), no match)",
            detail={
                "add": add_body if isinstance(add_body, dict) else None,
                "search_count": n,
                "target": target,
                "cleanup": cleanup_status,
                "record_id": record_id,
            },
        )
        return report

    cleanup_status = _probe_cleanup(sess, base, record_id, timeout)

    remote_note = " (remote allowed)" if remote_probe else ""
    report.add(
        "probe",
        "ok",
        f"add+search ok ({len(results)} result(s), hit target){remote_note}",
        detail={
            "add": add_body if isinstance(add_body, dict) else None,
            "search_count": len(results),
            "target": target,
            "remote": remote_probe,
            "cleanup": cleanup_status,
            "record_id": record_id,
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
