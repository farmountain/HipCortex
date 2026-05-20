# hipcortex

**Persistent causal memory for AI agents — LangChain, LlamaIndex, AutoGen, CrewAI.**

1.74ms p50 write latency. Temporal decay. Causal world model. Merkle-chained audit log. GDPR right-to-forget.

## Install

```bash
pip install hipcortex
```

## Quick start

```python
from hipcortex import HipCortexClient

client = HipCortexClient("http://localhost:3030")

# Store memory
client.add_memory(actor="alice", action="said", target="The meeting is at 3pm")

# Search (keyword or cosine similarity)
results = client.search("meeting time", limit=5)

# Stats
print(client.stats())

# GDPR forget
client.forget("alice")
```

## Framework integrations

```python
# LangChain — drop-in for ConversationBufferMemory
from hipcortex.langchain_memory import HipCortexMemory
memory = HipCortexMemory(session_id="user-42", url="http://localhost:3030")

# LlamaIndex
from hipcortex.llamaindex_storage import HipCortexChatStore
store = HipCortexChatStore(client=client)

# AutoGen
from hipcortex.adapters.autogen import HipCortexAutoGenMemory
mem = HipCortexAutoGenMemory(client=client, agent_id="researcher")
agent.register_hook("process_message_before_send", mem.on_message_sent)

# CrewAI
from hipcortex.adapters.crewai import HipCortexRememberTool, HipCortexRecallTool
tools = [HipCortexRememberTool(client=client), HipCortexRecallTool(client=client)]
```

## Start the server

```bash
# Self-hosted (single binary, zero deps)
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"

# Fly.io
fly deploy

# Live demo
https://hipcortex.fly.dev
```

**Full docs:** https://github.com/farmountain/HipCortex
