import math
import random
from typing import Any

def estimate_tokens(text: str) -> int:
    return max(1, math.floor(len(text) / 4))

class CodingSessionTraceBuilder:
    @staticmethod
    def build_trace(turns: int = 30, actor: str = "dev_alice") -> list[dict[str, Any]]:
        actions = ["read_file", "edit_file", "run_test", "compile_error", "refactor_decision"]
        records = []
        for i in range(turns):
            action = actions[i % len(actions)]
            payload = f"[Turn {i}] Actor {actor} performed {action} on module_step_{i} verifying complete state logic and semantic context validation across working tiers."
            tokens = estimate_tokens(payload)
            tier = "WorkingSet" if i < 10 else ("ShortTerm" if i < 20 else "LongTerm")
            records.append({
                "actor": actor,
                "action": action,
                "target": f"src/module_{i}.rs",
                "content": payload,
                "metadata": {"tier": tier, "tokens": tokens, "turn": i}
            })
        return records

class CausalDagBuilder:
    @staticmethod
    def build_chain(depth: int = 5, actor: str = "causal_agent") -> list[dict[str, Any]]:
        chain = []
        for i in range(depth):
            chain.append({
                "actor": actor,
                "action": f"causal_step_{i}",
                "target": f"node_{i}",
                "content": f"Causal observation {i} derived from prior state {i-1 if i > 0 else 'root'}",
                "metadata": {"step": i, "parent": f"causal_step_{i-1}" if i > 0 else None}
            })
        return chain
