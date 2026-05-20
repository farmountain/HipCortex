# hipcortex (TypeScript / JavaScript SDK)

Persistent causal memory for AI agents. Zero dependencies — uses native fetch.

## Install

```bash
npm install hipcortex
```

## Quick start

```typescript
import { HipCortexClient } from "hipcortex";

const client = new HipCortexClient({
  baseUrl: "http://localhost:3030",  // or https://hipcortex.fly.dev
  apiKey: "sk-your-key",            // optional
});

await client.addMemory({ actor: "user-42", action: "said", target: "Hello!" });
const { results } = await client.search({ query: "Hello", limit: 5 });
await client.forget("user-42");
const stats = await client.stats();
```

## Vercel AI SDK pattern

```typescript
import { HipCortexClient } from "hipcortex";
import { streamText } from "ai";

const memory = new HipCortexClient({ baseUrl: process.env.HIPCORTEX_URL! });

export async function POST(req: Request) {
  const { messages, userId } = await req.json();
  const history = await memory.getConversationHistory(userId, 20);
  const result = await streamText({ model: ..., messages: [...formatHistory(history.records), ...messages] });
  await memory.addHumanMessage(userId, messages.at(-1)?.content ?? "");
  return result.toDataStreamResponse();
}
```

## Server

```bash
# Docker (zero config)
docker run -p 3030:3030 ghcr.io/farmountain/hipcortex:latest

# Or build from source
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"
```
