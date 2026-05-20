/**
 * HipCortex context provider for Continue.dev
 *
 * Adds persistent memory to Continue.dev — recall past coding decisions,
 * architecture notes, bug fix reasoning, and project context.
 *
 * Install:
 *   1. Copy this file to your Continue config directory
 *   2. Add to ~/.continue/config.json (see README.md)
 *   3. Start HipCortex server: ./hipcortex  (or use hipcortex.fly.dev)
 *
 * Usage in Continue:
 *   @hipcortex why did we choose JWT?
 *   @hipcortex what bugs did we fix in auth?
 */

// Continue.dev ContextProvider interface (v0.9+)
interface ContextItem {
  name: string;
  description: string;
  content: string;
}

interface ContextProviderExtras {
  fullInput: string;
}

interface BaseContextProvider {
  get(query: string, extras: ContextProviderExtras, context: unknown): Promise<ContextItem[]>;
  getSubMenuItems?(): Promise<Array<{ title: string; description: string; id: string }>>;
}

// ─────────────────────────────────────────────────────────────────────────────

const HIPCORTEX_URL = process.env.HIPCORTEX_URL ?? "http://localhost:3030";
const HIPCORTEX_API_KEY = process.env.HIPCORTEX_API_KEY ?? "";
const DEFAULT_ACTOR = process.env.HIPCORTEX_ACTOR ?? "continue-dev";

function headers(): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  if (HIPCORTEX_API_KEY) h["X-Api-Key"] = HIPCORTEX_API_KEY;
  return h;
}

async function searchMemory(query: string, limit = 8): Promise<Array<{ score: number; record: Record<string, string> }>> {
  const resp = await fetch(`${HIPCORTEX_URL}/memory/search`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ query, limit }),
  });
  if (!resp.ok) return [];
  const data = await resp.json() as { results?: Array<{ score: number; record: Record<string, string> }> };
  return data.results ?? [];
}

async function addMemory(actor: string, action: string, target: string): Promise<void> {
  await fetch(`${HIPCORTEX_URL}/memory/add`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ actor, action, target, record_type: "Temporal" }),
  }).catch(() => { /* best-effort */ });
}

// ─────────────────────────────────────────────────────────────────────────────

export const HipCortexContextProvider: BaseContextProvider = {
  async get(query: string, extras: ContextProviderExtras): Promise<ContextItem[]> {
    const searchQuery = query || extras.fullInput;
    if (!searchQuery.trim()) return [];

    const results = await searchMemory(searchQuery, 10);
    if (results.length === 0) {
      return [{
        name: "hipcortex",
        description: "HipCortex memory",
        content: `No memories found for: "${searchQuery}"\n\nTip: Use @hipcortex remember <text> to store memories.`,
      }];
    }

    const lines = results.map((r, i) => {
      const rec = r.record;
      return `${i + 1}. [${rec.action ?? "?"}] ${rec.target ?? ""} (actor: ${rec.actor ?? "?"}, score: ${r.score.toFixed(2)})`;
    });

    return [{
      name: "hipcortex",
      description: `${results.length} memories matching "${searchQuery}"`,
      content: `## HipCortex Memories\n\n${lines.join("\n")}`,
    }];
  },
};

// Slash command adapter — /remember and /recall
export const HipCortexSlashCommands = {
  remember: {
    name: "remember",
    description: "Store a note in HipCortex memory",
    run: async function* (input: string) {
      await addMemory(DEFAULT_ACTOR, "noted", input.trim());
      yield `✓ Stored in HipCortex: "${input.trim()}"`;
    },
  },
  recall: {
    name: "recall",
    description: "Search HipCortex memory",
    run: async function* (input: string) {
      const results = await searchMemory(input.trim(), 5);
      if (!results.length) {
        yield "No memories found.";
        return;
      }
      const lines = results.map((r) => `• [${r.record.action}] ${r.record.target}`).join("\n");
      yield `**HipCortex memories:**\n${lines}`;
    },
  },
};

export default HipCortexContextProvider;
