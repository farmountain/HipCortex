from .models import AcceptanceContract


def compile_goal(goal: str) -> AcceptanceContract:
    """Create a conservative initial contract from a developer goal.

    LLM-backed compilation can replace this function later; the contract is
    intentionally explicit so downstream execution never depends on prose.
    """
    return AcceptanceContract(
        goal=goal,
        acceptance_criteria=[
            "The requested behavior is implemented.",
            "Existing relevant tests continue to pass.",
            "A regression test exists when the change is bug-related.",
            "The final diff is limited to the requested scope.",
        ],
        stop_conditions=[
            "Do not modify files outside the repository.",
            "Stop when validation cannot establish the acceptance criteria.",
        ],
        test_commands=[],
    )
