"""Headless intent runner — polls /intent/open and posts receipts.

Enables 3-month autonomy: a process that keeps the probe loop alive
after the IDE is closed, without any LLM involvement.
"""

from __future__ import annotations

import logging
import os
import subprocess
import time
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional
from urllib.parse import urlparse

try:
    import requests as _requests
    _HAS_REQUESTS = True
except ImportError:
    _HAS_REQUESTS = False

logger = logging.getLogger("hipcortex.runner")

# Shell commands allowed for shell:// sensor paths (allowlist, no arbitrary exec)
_SHELL_ALLOWLIST = {
    "ping": ["ping", "-c", "1", "-W", "2"],
    "curl_head": ["curl", "-sI", "--max-time", "5"],
    "nc_port": ["nc", "-zw2"],
}


class IntentRunner:
    """Poll /intent/open, dispatch probe, post receipt.

    Args:
        actor: Actor label used to filter intents (e.g. "hipcortex-runner").
        base_url: HipCortex server base URL (default: http://localhost:3030).
        interval_secs: Poll interval in seconds (default: 30).
        dry_run: If True, log actions but do not POST receipts.
    """

    def __init__(
        self,
        actor: str = "hipcortex-runner",
        base_url: str = "http://localhost:3030",
        interval_secs: float = 30.0,
        dry_run: bool = False,
    ) -> None:
        self.actor = actor
        self.base_url = base_url.rstrip("/")
        self.interval_secs = interval_secs
        self.dry_run = dry_run

    # ── public ────────────────────────────────────────────────────────────────

    def run_forever(self) -> None:
        """Block forever, polling and executing intents."""
        logger.info("HipCortex runner started. actor=%s url=%s interval=%ss dry_run=%s",
                    self.actor, self.base_url, self.interval_secs, self.dry_run)
        while True:
            try:
                self.poll_and_execute()
            except Exception as exc:  # noqa: BLE001
                logger.warning("poll error: %s", exc)
            time.sleep(self.interval_secs)

    def poll_and_execute(self, actor: Optional[str] = None) -> List[Dict[str, Any]]:
        """Poll open intents, execute each, return list of receipt payloads sent."""
        af = actor or self.actor
        intents = self._fetch_open_intents(af)
        results = []
        now_ms = _now_ms()
        for intent in intents:
            intent_id = intent.get("id") or intent.get("intent_id")
            if not intent_id:
                continue
            deadline = intent.get("deadline_ms")
            if deadline and now_ms > deadline:
                logger.debug("skip expired intent %s", intent_id)
                continue
            target = intent.get("target_entity", "")
            sensor = intent.get("sensor_path", "default")
            receipt = self.execute_probe(intent_id, target, sensor)
            results.append(receipt)
            self._post_receipt(af, receipt)
        return results

    def execute_probe(self, intent_id: str, target: str, sensor_path: str) -> Dict[str, Any]:
        """Dispatch probe based on sensor_path hint; return receipt dict."""
        sensor = (sensor_path or "default").lower()
        try:
            if sensor == "filesystem":
                obs = _probe_filesystem(target)
            elif sensor == "http":
                obs = _probe_http(target)
            elif sensor.startswith("shell:"):
                obs = _probe_shell(sensor[6:], target)
            else:
                obs = {"reachable": True, "sensor": "default"}
            ok = True
        except Exception as exc:  # noqa: BLE001
            obs = {"error": str(exc)}
            ok = False

        return {
            "intent_id": intent_id,
            "ok": ok,
            "observation": obs,
            "sensor_path": sensor_path,
        }

    # ── private ───────────────────────────────────────────────────────────────

    def _fetch_open_intents(self, actor: str) -> List[Dict[str, Any]]:
        url = f"{self.base_url}/intent/open"
        params = {"actor": actor} if actor else {}
        if not _HAS_REQUESTS:
            raise RuntimeError("requests package required: pip install requests")
        resp = _requests.get(url, params=params, timeout=10)
        resp.raise_for_status()
        data = resp.json()
        return data.get("intents", [])

    def _post_receipt(self, actor: str, receipt: Dict[str, Any]) -> None:
        payload = {
            "actor": actor,
            "intent_id": receipt["intent_id"],
            "ok": receipt["ok"],
            "observation": receipt.get("observation", {}),
            "sensor_path": receipt.get("sensor_path", "default"),
        }
        if self.dry_run:
            logger.info("[dry-run] POST /intent/receipt %s", payload)
            return
        url = f"{self.base_url}/intent/receipt"
        resp = _requests.post(url, json=payload, timeout=10)
        resp.raise_for_status()
        logger.debug("receipt posted intent_id=%s ok=%s", receipt["intent_id"], receipt["ok"])


# ── probe implementations ─────────────────────────────────────────────────────

def _probe_filesystem(target: str) -> Dict[str, Any]:
    stat = os.stat(target)
    return {
        "exists": True,
        "size": stat.st_size,
        "mtime": stat.st_mtime,
        "sensor": "filesystem",
    }


def _probe_http(target: str) -> Dict[str, Any]:
    if not _HAS_REQUESTS:
        raise RuntimeError("requests package required")
    parsed = urlparse(target)
    if parsed.scheme not in ("http", "https"):
        raise ValueError(f"http probe requires http/https URL, got: {target!r}")
    resp = _requests.get(target, timeout=10, allow_redirects=True)
    return {
        "status_code": resp.status_code,
        "ok": resp.ok,
        "content_length": len(resp.content),
        "sensor": "http",
    }


def _probe_shell(command_key: str, target: str) -> Dict[str, Any]:
    cmd_prefix = _SHELL_ALLOWLIST.get(command_key)
    if cmd_prefix is None:
        raise ValueError(f"shell command not in allowlist: {command_key!r}. Allowed: {list(_SHELL_ALLOWLIST)}")
    cmd = cmd_prefix + ([target] if target else [])
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    return {
        "returncode": result.returncode,
        "ok": result.returncode == 0,
        "sensor": f"shell:{command_key}",
    }


def _now_ms() -> int:
    return int(datetime.now(timezone.utc).timestamp() * 1000)
