# HipCortex TypeScript & Node.js SDK (`v0.5.1`)

**Persistent causal topological memory, recursive Bayesian world-model prediction (`/worldmodel/rollout`), and automatic FSM skill compilation for autonomous AI agents.**

Runs against the standalone local Rust binary (`http://127.0.0.1:3030`) with `0.48–0.61 ms p50` write latency, SHA-256 Merkle audit chains, and Headroom context budgeting (`59–88% token savings`). Zero external dependencies — uses native `fetch`.

---

## 🚀 Installation

```bash
npm install hipcortex
```

> **Installer note:** **Python is the canonical installer** (`pip install hipcortex` → `hipcortex install`).  
> This npm package is a **TypeScript/Node HTTP client only** — no framework scaffold wizard, no binary download, no IDE MCP registration. Use the Python CLI for install/scaffold; use this package for Node apps that talk to an existing HipCortex server.
>
> **Version matrix:** npm client **0.5.1** talks to server **0.5.1** (includes `/worldmodel/rollout`, live_beliefs, predict). VS Code extension is **0.5.7** with 10 LM tools — [docs/channels.md](../../docs/channels.md).

---

## ⚡ Quick Start: Multi-Tier Memory & World Model Prediction

```typescript
import { HipCortexClient } from "hipcortex";

const client = new HipCortexClient({
  baseUrl: "http://127.0.0.1:3030", // or process.env.HIPCORTEX_URL
});

// 1. Multi-Tier Memory Ingestion (5 verified memory tiers)
await client.addMemory({
  actor: "agent",
  action: "configured",
  target: "jwt_token_ttl=3600",
  record_type: "Working", // Mapped natively to Temporal tier
  priority: "high",
});

await client.addMemory({
  actor: "agent",
  action: "established",
  target: "All database migrations must run inside transactions",
  record_type: "Semantic", // Mapped natively to Symbolic/LongTerm tier
  priority: "pinned",
});

// 2. Check SelfModel Execution Capacity Gates before running risky actions
if (await client.canExecute("rollout")) {
  console.log("Health check passed — engine ready for simulation");
}

// 3. World Model Trajectory Rollout Prediction (POST /worldmodel/rollout)
const rolloutResult = await client.rollout({
  initial_state: { db_status: "locked", active_tx: 1 },
  actions: ["rollback_tx", "release_lock", "retry_migration"],
});
console.log("Predicted outcome:", rolloutResult);
```

---

## 🧠 Proactive CodeAct Harness & Vercel AI SDK Pattern

Query the topological graph (`Personalized PageRank`) directly inside your LLM request pipeline:

```typescript
import { HipCortexClient } from "hipcortex";
import { streamText } from "ai";

const memory = new HipCortexClient({ baseUrl: process.env.HIPCORTEX_URL! });

export async function POST(req: Request) {
  const { messages, userId } = await req.json();
  
  // Retrieve Top-K causal memory context
  const history = await memory.queryMemory({ actor: userId, limit: 10 });
  
  const result = await streamText({
    model: yourModelInstance,
    messages: [
      { role: "system", content: `Active Causal Context:\n${JSON.stringify(history.records)}` },
      ...messages
    ]
  });
  
  // Store user prompt asynchronously into causal memory
  const lastUserMsg = messages.at(-1)?.content ?? "";
  if (lastUserMsg) {
    await memory.addMemory({ actor: userId, action: "prompted", target: lastUserMsg, record_type: "Temporal" });
  }
  
  return result.toDataStreamResponse();
}
```

---

## 🔗 Topological Memory Foundation (`TMF`) Graph Methods

```typescript
// Link two memories explicitly in the CausalTopoGraph
await client.linkMemories({
  source_id: "rec-uuid-1",
  target_id: "rec-uuid-2",
  relation: "caused_by"
});

// Perform Personalized PageRank (PPR α=0.85) related search from a seed record
const { results } = await client.searchRelated("rec-uuid-1", 10);
console.log("Topologically related memories:", results);
```

---

## 📊 Server Statistics & GDPR Right-to-Forget

```typescript
// Get server diagnostics & Merkle hash chain status
const stats = await client.stats();
console.log("Memory statistics:", stats);

// Erase all records for a specific actor (GDPR right-to-forget)
await client.forget("agent");
```
