# HipCortex Python SDK (`v0.5.0`)

**Persistent causal topological memory, recursive Bayesian world-model prediction (`/worldmodel/rollout`), and automatic FSM skill compilation for autonomous AI agents.**

Runs against the standalone local Rust binary (`http://127.0.0.1:3030`) with `0.48–0.61 ms p50` write latency, SHA-256 Merkle audit chains, and Headroom context budgeting (`59–88% token savings`).

---

## 🚀 Installation

```bash
pip install hipcortex
```

---

## ⚡ Quick Start: Multi-Tier Memory & World Model Prediction

```python
from hipcortex import HipCortexClient

client = HipCortexClient(base_url="http://127.0.0.1:3030")

# 1. Multi-Tier Memory Ingestion (5 verified memory tiers)
client.add_memory(
    actor="agent",
    action="configured",
    target="jwt_token_ttl=3600",
    record_type="Working",  # Mapped natively to Temporal tier
    priority="high"
)

client.add_memory(
    actor="agent",
    action="established",
    target="All database migrations must run inside transactions",
    record_type="Semantic", # Mapped natively to Symbolic/LongTerm tier
    priority="pinned"
)

# 2. Check SelfModel Execution Capacity Gates before running risky actions
if client.can_execute("rollout"):
    print("Health check passed — engine ready for simulation")

# 3. World Model Trajectory Rollout Prediction (POST /worldmodel/rollout)
rollout_result = client.rollout(
    initial_state={"db_status": "locked", "active_tx": 1},
    actions=["rollback_tx", "release_lock", "retry_migration"]
)
print("Predicted outcome:", rollout_result)
```

---

## 🧠 Proactive CodeAct Harness & Live Beliefs

For autonomous agent harnesses (`LangChain`, `LlamaIndex`, `CrewAI`, `AutoGen`), query the topological substrate before reasoning:

```python
# Query live causal beliefs and Top-K topological context (Personalized PageRank)
context = client.context(query="database migrations", actor="agent", limit=5)
print("Active Topological Context:\n", context)

# Store multi-hop reasoning or CoT reflection
client.add_memory(
    actor="agent",
    action="reflected",
    target="Migration failed due to missing index on users.email",
    record_type="Causal"
)
```

---

## 🧩 Framework Integrations

### LangChain (`ConversationBufferMemory` drop-in)
```python
from hipcortex.langchain_memory import HipCortexMemory

memory = HipCortexMemory(session_id="session-42", url="http://127.0.0.1:3030")
```

### LlamaIndex (`ChatStore`)
```python
from hipcortex.llamaindex_storage import HipCortexStorageContext

storage_context = HipCortexStorageContext.from_defaults(client=client)
```

### CrewAI (`Remember` & `Recall` Tools)
```python
from hipcortex.adapters.crewai import HipCortexRememberTool, HipCortexRecallTool

tools = [HipCortexRememberTool(client=client), HipCortexRecallTool(client=client)]
```

### AutoGen (`Memory` Hook)
```python
from hipcortex.adapters.autogen import HipCortexAutoGenMemory

mem = HipCortexAutoGenMemory(client=client, agent_id="researcher")
agent.register_hook("process_message_before_send", mem.on_message_sent)
```

---

## 📊 Server Statistics & GDPR Right-to-Forget

```python
# Get Merkle hash chain status and tier counts
stats = client.stats()
print("Memory statistics:", stats)

# Erase all records for a specific actor (GDPR compliant)
client.forget("agent")
```
