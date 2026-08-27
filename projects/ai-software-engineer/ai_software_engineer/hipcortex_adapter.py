from typing import Any, Dict, List, Optional


class HipCortexAdapter:
    """Application boundary around the HipCortex SDK/server.

    The adapter keeps the AI Software Engineer independent from HipCortex
    implementation details while exposing the state needed by the loop.
    """

    def __init__(self, substrate: Optional[Any] = None, actor: str = "ai-software-engineer"):
        self.substrate = substrate
        self.actor = actor
        self.local_events: List[Dict[str, Any]] = []

    def remember(self, action: str, target: str, **metadata: Any) -> None:
        event = {"actor": self.actor, "action": action, "target": target, **metadata}
        self.local_events.append(event)
        if self.substrate is not None and hasattr(self.substrate, "add_memory"):
            self.substrate.add_memory(actor=self.actor, action=action, target=target)

    def recall(self, query: str, limit: int = 5) -> List[Dict[str, Any]]:
        if self.substrate is not None and hasattr(self.substrate, "experience_search"):
            return self.substrate.experience_search(self.actor, query)[:limit]
        return [e for e in self.local_events if query.lower() in str(e).lower()][-limit:]
