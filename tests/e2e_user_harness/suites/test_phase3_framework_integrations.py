import ast
from pathlib import Path
import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient

@pytest.mark.integration
def test_sdk_client_and_crewai_adapter(raw_client: HarnessHttpxClient):
    """Verify CrewAI adapter works cleanly with live harness endpoints."""
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import (
        HipCortexRememberTool,
        HipCortexRecallTool,
        HipCortexForgetTool,
    )
    
    client = HipCortexClient(raw_client.base_url)
    assert client.health() is True
    
    remember_tool = HipCortexRememberTool(client=client, agent_id="test_crew_agent")
    assert remember_tool.name == "hipcortex_remember"
    
    recall_tool = HipCortexRecallTool(client=client, agent_id="test_crew_agent")
    assert recall_tool.name == "hipcortex_recall"
    
    forget_tool = HipCortexForgetTool(client=client, agent_id="test_crew_agent")
    assert forget_tool.name == "hipcortex_forget"

@pytest.mark.integration
def test_autogen_adapter_structure(raw_client: HarnessHttpxClient):
    """Verify AutoGen memory adapter definitions and initialization."""
    from hipcortex import HipCortexClient
    from hipcortex.adapters.autogen import HipCortexAutoGenMemory
    
    client = HipCortexClient(raw_client.base_url)
    memory = HipCortexAutoGenMemory(client=client, agent_id="memory_companion")
    assert hasattr(memory, "add")
    assert hasattr(memory, "query")
    assert hasattr(memory, "store")
    assert hasattr(memory, "recall")


@pytest.mark.integration
def test_framework_bridge_scripts_syntax():
    """Verify all 5 generated framework bridge scripts are syntactically valid and contain expected targets."""
    workspace_root = Path(__file__).parent.parent.parent.parent
    bridge_files = [
        "hipcortex_crewai.py",
        "hipcortex_llamaindex.py",
        "hipcortex_langchain.py",
        "hipcortex_autogen.py",
        "hipcortex_dspy.py",
    ]
    
    for filename in bridge_files:
        filepath = workspace_root / filename
        assert filepath.exists(), f"Missing bridge file: {filename}"
        content = filepath.read_text(encoding="utf-8")
        # Ensure valid Python AST
        ast.parse(content, filename=str(filepath))
        assert "hipcortex" in content.lower(), f"Bridge file {filename} missing hipcortex reference"
