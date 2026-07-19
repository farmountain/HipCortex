"""Shared identity factories for framework adapters.

Package-first helpers so LangChain / CrewAI / AutoGen (and examples) share
one path: load settings → client + default session/actor → optional live_beliefs.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from ..client import HipCortexClient


def client_from_settings(
    project_root: Optional[Any] = None,
    **client_kw: Any,
) -> HipCortexClient:
    """Build :class:`HipCortexClient` from :func:`hipcortex.config.load_settings`.

    ``base_url`` is taken from settings when not passed in ``client_kw``.
    Extra kwargs (e.g. ``timeout``) forward to the client constructor.
    """
    from ..config import load_settings

    if "base_url" not in client_kw or client_kw.get("base_url") is None:
        settings = load_settings(project_root)
        client_kw["base_url"] = settings.url
    return HipCortexClient(**client_kw)


def default_session_id(project_root: Optional[Any] = None) -> str:
    """Resolved default actor from config, or ``\"default\"`` when unset."""
    from ..config import get_default_actor

    actor = get_default_actor(project_root)
    if actor is None or not str(actor).strip():
        return "default"
    return str(actor).strip()


def _format_belief_items(data: Dict[str, Any], limit: int) -> List[str]:
    """Extract short lines from a live_beliefs payload (best-effort)."""
    lines: List[str] = []

    summary = data.get("summary")
    if isinstance(summary, str) and summary.strip():
        lines.append(summary.strip())

    # Common nested shapes from /memory/live_beliefs
    facts = data.get("symbolic_facts") or data.get("facts") or {}
    if isinstance(facts, dict):
        nodes = facts.get("nodes") or facts.get("items") or []
        if isinstance(nodes, list):
            for node in nodes:
                if len(lines) >= limit:
                    break
                if isinstance(node, dict):
                    label = (
                        node.get("label")
                        or node.get("name")
                        or node.get("target")
                        or node.get("id")
                        or str(node)
                    )
                    lines.append(f"fact: {label}")
                else:
                    lines.append(f"fact: {node}")
        elif facts and not nodes:
            for k, v in list(facts.items())[: max(0, limit - len(lines))]:
                if k in ("nodes", "items"):
                    continue
                lines.append(f"fact: {k}={v}")

    hyps = data.get("hypotheses") or data.get("hyp") or []
    if isinstance(hyps, list):
        for h in hyps:
            if len(lines) >= limit:
                break
            if isinstance(h, dict):
                text = h.get("hypothesis") or h.get("text") or h.get("target") or str(h)
                conf = h.get("confidence")
                if conf is not None:
                    lines.append(f"hyp: {text} (conf={conf})")
                else:
                    lines.append(f"hyp: {text}")
            else:
                lines.append(f"hyp: {h}")
    elif isinstance(hyps, str) and hyps.strip() and len(lines) < limit:
        lines.append(f"hyp: {hyps.strip()}")

    preds = data.get("predictions") or data.get("world") or []
    if isinstance(preds, list):
        for p in preds:
            if len(lines) >= limit:
                break
            if isinstance(p, dict):
                text = p.get("prediction") or p.get("state") or p.get("target") or str(p)
                lines.append(f"pred: {text}")
            else:
                lines.append(f"pred: {p}")
    elif isinstance(preds, dict) and len(lines) < limit:
        state = preds.get("state") or preds.get("from_state")
        if state:
            lines.append(f"pred: {state}")

    # Flat leftover keys sometimes used in harness mocks
    for key in ("state", "hyp", "pred"):
        if key in data and not isinstance(data[key], (dict, list)) and len(lines) < limit:
            val = data[key]
            if val is not None and str(val).strip():
                # avoid duplicating hyp already handled
                if key == "hyp" and any(l.startswith("hyp:") for l in lines):
                    continue
                lines.append(f"{key}: {val}")

    return lines[:limit]


def bootstrap_live_beliefs(
    client: HipCortexClient,
    actor: Optional[str] = None,
    limit: int = 5,
) -> str:
    """Fetch live beliefs and return a short context string (no network raise).

    Args:
        client: Configured HTTP client.
        actor: Optional actor/session label included in the header line.
        limit: Max belief lines after optional summary.

    Returns:
        Multi-line summary for prompt injection, or empty string on failure /
        empty substrate.
    """
    try:
        data = client.live_beliefs()
    except Exception:
        return ""

    if not isinstance(data, dict):
        return str(data) if data else ""

    lines = _format_belief_items(data, limit=max(1, limit))
    if not lines:
        return ""

    header = "[HipCortex live_beliefs"
    if actor:
        header += f" actor={actor}"
    header += "]"
    return header + "\n" + "\n".join(lines)
