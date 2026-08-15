"""Passive observer unit tests — Tasks 1-4."""
from unittest.mock import MagicMock


# ── Task 1: LangChain callback handler ─────────────────────────────────────────

def test_langchain_callback_handler_importable():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    assert HipCortexCallbackHandler is not None


def test_langchain_callback_on_llm_start_calls_add_memory():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    mock_client = MagicMock()
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")
    handler.on_llm_start(serialized={}, prompts=["Hello world"], run_id="r1")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "lc-agent"
    assert kwargs["action"] == "llm_start"


def test_langchain_callback_on_llm_end_calls_add_memory():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    mock_client = MagicMock()
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")

    class FakeGeneration:
        text = "done"

    class FakeLLMResult:
        generations = [[FakeGeneration()]]

    handler.on_llm_end(response=FakeLLMResult(), run_id="r1")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["action"] == "llm_end"


def test_langchain_callback_never_raises_on_client_error():
    from hipcortex.langchain_memory import HipCortexCallbackHandler
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = RuntimeError("network down")
    handler = HipCortexCallbackHandler(client=mock_client, actor="lc-agent")
    handler.on_llm_start(serialized={}, prompts=["test"], run_id="r1")
    handler.on_tool_start(serialized={"name": "search"}, input_str="q", run_id="r2")


# ── Task 2: CrewAI crew observer ───────────────────────────────────────────────

def test_crewai_observer_importable():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    assert HipCortexCrewObserver is not None


def test_crewai_observer_step_callback_is_callable():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    cb = obs.step_callback
    assert callable(cb), "step_callback must return a callable"


def test_crewai_observer_step_callback_captures_action():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    action = MagicMock()
    action.tool = "SearchTool"
    action.tool_input = "python memory"
    action.result = "found docs"
    obs.step_callback(action)
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "crew-test"
    assert kwargs["action"] == "crew_step"
    assert "SearchTool" in kwargs["target"]


def test_crewai_observer_step_callback_never_raises():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = ConnectionError("timeout")
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    obs.step_callback(MagicMock())


def test_crewai_observer_inject_context_idempotent():
    from hipcortex.adapters.crewai import HipCortexCrewObserver
    mock_client = MagicMock()
    mock_client.query_memory.return_value = []
    obs = HipCortexCrewObserver(client=mock_client, actor="crew-test")
    mock_crew = MagicMock()
    obs.inject_context(mock_crew)
    obs.inject_context(mock_crew)
    assert mock_client.query_memory.call_count == 1, "inject_context must be idempotent"


# ── Task 3: AutoGen observer ───────────────────────────────────────────────────

def test_autogen_observer_importable():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    assert HipCortexAutoGenObserver is not None


def test_autogen_observer_on_message_received_captures():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_message_received(sender="UserProxy", content="What is 2+2?", role="user")
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["actor"] == "autogen-agent"
    assert kwargs["action"] == "message_received"
    assert "What is 2+2?" in kwargs["target"]


def test_autogen_observer_on_function_call_captures():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_function_call(name="web_search", arguments={"query": "python"})
    mock_client.add_memory.assert_called_once()
    kwargs = mock_client.add_memory.call_args[1]
    assert kwargs["action"] == "function_call"
    assert "web_search" in kwargs["target"]


def test_autogen_observer_never_raises():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    mock_client.add_memory.side_effect = OSError("socket error")
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    obs.on_message_received(sender="A", content="hello", role="user")
    obs.on_function_call(name="fn", arguments={})


def test_autogen_observer_v03_send_hook():
    from hipcortex.adapters.autogen import HipCortexAutoGenObserver
    mock_client = MagicMock()
    obs = HipCortexAutoGenObserver(client=mock_client, actor="autogen-agent")
    hook = obs.make_v03_send_hook()
    hook({"content": "Hello there", "role": "assistant"})
    mock_client.add_memory.assert_called_once()


# ── Task 4: VSIX passiveCapture config ─────────────────────────────────────────

def test_vsix_package_json_has_passive_capture_config():
    import json
    import os
    pkg_path = os.path.join(
        os.path.dirname(__file__),
        "../../../vscode-extension/package.json",
    )
    with open(pkg_path) as f:
        pkg = json.load(f)
    props = (
        pkg.get("contributes", {})
        .get("configuration", {})
        .get("properties", {})
    )
    assert "hipcortex.passiveCapture" in props, \
        "package.json missing hipcortex.passiveCapture config"
    config = props["hipcortex.passiveCapture"]
    assert config["type"] == "boolean"
    assert config["default"] is True


def test_vsix_extension_ts_has_passive_capture_guard():
    import os
    ts_path = os.path.join(
        os.path.dirname(__file__),
        "../../../vscode-extension/src/extension.ts",
    )
    with open(ts_path) as f:
        content = f.read()
    assert "passiveCapture" in content, "extension.ts missing passiveCapture guard"
    assert "onDidWriteTerminalData" in content, "extension.ts missing terminal listener"
