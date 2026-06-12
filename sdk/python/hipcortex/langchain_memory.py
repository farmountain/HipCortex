"""LangChain-compatible memory backed by HipCortex.

Drop-in replacement for ``ConversationBufferMemory`` in any LangChain chain.

Usage::

    from hipcortex import HipCortexClient
    from hipcortex.langchain_memory import HipCortexMemory
    from langchain.chains import ConversationChain
    from langchain.chat_models import ChatOpenAI

    client = HipCortexClient(base_url="http://127.0.0.1:3030")
    memory = HipCortexMemory(client=client, session_id="user-42")

    chain = ConversationChain(llm=ChatOpenAI(), memory=memory)
    chain.predict(input="Hello!")
"""

from __future__ import annotations

from typing import Any, Dict, List

from .client import HipCortexClient

try:
    from langchain.schema import BaseMemory
    from langchain.schema.messages import AIMessage, HumanMessage, BaseMessage
    _LANGCHAIN_AVAILABLE = True
except ImportError:
    _LANGCHAIN_AVAILABLE = False
    # Provide a no-op base so the class can be imported without LangChain installed
    class BaseMemory:  # type: ignore[no-redef]
        def save_context(self, inputs: Dict, outputs: Dict) -> None: ...
        def load_memory_variables(self, inputs: Dict) -> Dict: ...
        def clear(self) -> None: ...


class HipCortexMemory(BaseMemory):
    """LangChain ``BaseMemory`` implementation backed by HipCortex REST API.

    Args:
        client:     Configured :class:`~hipcortex.client.HipCortexClient`.
        session_id: Scopes all memory reads/writes to this actor identifier.
        memory_key: Key used when injecting history into the chain's prompt vars
                    (default: ``"history"``).
        human_prefix: Label prepended to human turns in the history string.
        ai_prefix:    Label prepended to AI turns in the history string.
        max_records:  Max records fetched per ``load_memory_variables`` call.
    """

    # Not using pydantic model here to stay LangChain version-agnostic

    def __init__(
        self,
        client: HipCortexClient,
        session_id: str = "default",
        memory_key: str = "history",
        human_prefix: str = "Human",
        ai_prefix: str = "AI",
        max_records: int = 50,
    ) -> None:
        self.client = client
        self.session_id = session_id
        self.memory_key = memory_key
        self.human_prefix = human_prefix
        self.ai_prefix = ai_prefix
        self.max_records = max_records

    # Required by BaseMemory -------------------------------------------------

    @property
    def memory_variables(self) -> List[str]:
        return [self.memory_key]

    def load_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Fetch conversation history from HipCortex and format as a string."""
        records = self.client.get_conversation_history(
            self.session_id, limit=self.max_records
        )
        # Sort oldest-first
        records.sort(key=lambda r: r.get("timestamp", ""))

        lines: List[str] = []
        for rec in records:
            action = rec.get("action", "")
            target = rec.get("target", "")
            if action == "human_message":
                lines.append(f"{self.human_prefix}: {target}")
            elif action == "ai_message":
                lines.append(f"{self.ai_prefix}: {target}")

        return {self.memory_key: "\n".join(lines)}

    def save_context(self, inputs: Dict[str, Any], outputs: Dict[str, Any]) -> None:
        """Persist a human→AI exchange to HipCortex."""
        human_text = inputs.get("input") or inputs.get("human_input") or ""
        ai_text = outputs.get("output") or outputs.get("response") or ""
        if human_text:
            self.client.add_human_message(self.session_id, str(human_text))
        if ai_text:
            self.client.add_ai_message(self.session_id, str(ai_text))

    def clear(self) -> None:
        """Invoke GDPR forget for the current session."""
        self.client.forget(self.session_id)

    # Optional helper --------------------------------------------------------

    def get_messages(self) -> List[Any]:
        """Return conversation history as LangChain ``BaseMessage`` objects.

        Requires LangChain to be installed; raises ``ImportError`` otherwise.
        """
        if not _LANGCHAIN_AVAILABLE:
            raise ImportError("langchain is required: pip install langchain")
        records = self.client.get_conversation_history(
            self.session_id, limit=self.max_records
        )
        records.sort(key=lambda r: r.get("timestamp", ""))
        messages: List[BaseMessage] = []
        for rec in records:
            target = rec.get("target", "")
            if rec.get("action") == "human_message":
                messages.append(HumanMessage(content=target))
            elif rec.get("action") == "ai_message":
                messages.append(AIMessage(content=target))
        return messages


class AsyncHipCortexMemory:
    """Async-native LangChain BaseMemory backed by AsyncHipCortexClient.

    Use with async LangChain chains (LangChain 0.2+, FastAPI, Django async).
    Implements aload_memory_variables and asave_context as coroutines.

    Usage::

        from hipcortex import AsyncHipCortexClient
        from hipcortex.langchain_memory import AsyncHipCortexMemory

        async def chat(user_input: str):
            client = AsyncHipCortexClient("http://127.0.0.1:3030")
            memory = AsyncHipCortexMemory(client=client, session_id="user-42")
            history = await memory.aload_memory_variables({})
            await memory.asave_context({"input": user_input}, {"output": ai_response})
    """

    def __init__(
        self,
        client: "Any",
        session_id: str = "default",
        memory_key: str = "history",
        human_prefix: str = "Human",
        ai_prefix: str = "AI",
        max_limit: int = 50,
    ) -> None:
        self._client = client
        self.session_id = session_id
        self.memory_key = memory_key
        self.human_prefix = human_prefix
        self.ai_prefix = ai_prefix
        self.max_limit = max_limit

    @property
    def memory_variables(self) -> List[str]:
        return [self.memory_key]

    async def aload_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Fetch conversation history as formatted string (async)."""
        records = await self._client.get_conversation_history(
            self.session_id, limit=self.max_limit
        )
        records.sort(key=lambda r: r.get("timestamp", ""))
        lines: List[str] = []
        for rec in records:
            action = rec.get("action", "")
            target = rec.get("target", "")
            if action == "human_message":
                lines.append(f"{self.human_prefix}: {target}")
            elif action == "ai_message":
                lines.append(f"{self.ai_prefix}: {target}")
        return {self.memory_key: "\n".join(lines)}

    async def asave_context(
        self, inputs: Dict[str, Any], outputs: Dict[str, Any]
    ) -> None:
        """Persist a human->AI exchange asynchronously."""
        human_text = inputs.get("input") or inputs.get("human_input") or ""
        ai_text    = outputs.get("output") or outputs.get("response") or ""
        if human_text:
            await self._client.add_human_message(self.session_id, str(human_text))
        if ai_text:
            await self._client.add_ai_message(self.session_id, str(ai_text))

    async def aclear(self) -> None:
        """GDPR forget for this session."""
        await self._client.forget(self.session_id)

    def load_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Sync fallback using asyncio.run()."""
        import asyncio
        import concurrent.futures
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    return pool.submit(asyncio.run, self.aload_memory_variables(inputs)).result()
            return loop.run_until_complete(self.aload_memory_variables(inputs))
        except RuntimeError:
            return asyncio.run(self.aload_memory_variables(inputs))

    def save_context(self, inputs: Dict[str, Any], outputs: Dict[str, Any]) -> None:
        """Sync fallback using asyncio.run()."""
        import asyncio
        import concurrent.futures
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    pool.submit(asyncio.run, self.asave_context(inputs, outputs)).result()
                return
            loop.run_until_complete(self.asave_context(inputs, outputs))
        except RuntimeError:
            asyncio.run(self.asave_context(inputs, outputs))

    def clear(self) -> None:
        """Sync fallback using asyncio.run()."""
        import asyncio
        import concurrent.futures
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    pool.submit(asyncio.run, self.aclear()).result()
                return
            loop.run_until_complete(self.aclear())
        except RuntimeError:
            asyncio.run(self.aclear())
