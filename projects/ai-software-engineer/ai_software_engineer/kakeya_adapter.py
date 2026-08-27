from typing import Any, Dict, List, Optional


class KakeyaAdapter:
    """Boundary for KARAM abstraction/reasoning/action operators.

    The initial adapter is intentionally thin. Native KARAM operators can be
    plugged in without changing the application-level control loop.
    """

    def __init__(self, engine: Optional[Any] = None):
        self.engine = engine

    def abstract(self, observation: str) -> Dict[str, Any]:
        if self.engine is not None and hasattr(self.engine, "abstract"):
            return self.engine.abstract(observation)
        return {"type": "repository_observation", "content": observation}

    def propose_actions(self, state: Dict[str, Any], goal: str) -> List[Dict[str, Any]]:
        if self.engine is not None and hasattr(self.engine, "propose_actions"):
            return self.engine.propose_actions(state, goal)
        return []

    def validate(self, state: Dict[str, Any], evidence: List[Dict[str, Any]]) -> bool:
        if self.engine is not None and hasattr(self.engine, "validate"):
            return bool(self.engine.validate(state, evidence))
        return all(item.get("returncode", 1) == 0 for item in evidence) if evidence else False
