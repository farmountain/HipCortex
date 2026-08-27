from dataclasses import dataclass, field
from typing import List, Dict


@dataclass
class AcceptanceContract:
    goal: str
    acceptance_criteria: List[str]
    stop_conditions: List[str] = field(default_factory=list)
    test_commands: List[str] = field(default_factory=list)


@dataclass
class EngineeringState:
    repo: str
    contract: AcceptanceContract
    observations: List[Dict[str, str]] = field(default_factory=list)
    actions: List[Dict[str, str]] = field(default_factory=list)
    failures: List[Dict[str, str]] = field(default_factory=list)
    iteration: int = 0
    status: str = "initialized"
