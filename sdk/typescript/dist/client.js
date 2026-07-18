"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.HipCortexClient = void 0;
class HipCortexClient {
    constructor(options = {}) {
        this.baseUrl = (options.baseUrl ?? "http://localhost:3030").replace(/\/$/, "");
        this.headers = { "Content-Type": "application/json" };
        if (options.apiKey)
            this.headers["X-Api-Key"] = options.apiKey;
        this.timeout = options.timeout ?? 10000;
    }
    async request(method, path, body, params) {
        let url = `${this.baseUrl}${path}`;
        if (params) {
            const entries = Object.entries(params).filter(([, v]) => v !== undefined && v !== null);
            if (entries.length > 0) {
                url += "?" + new URLSearchParams(entries.map(([k, v]) => [k, String(v)])).toString();
            }
        }
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);
        try {
            const resp = await fetch(url, {
                method,
                headers: this.headers,
                body: body !== undefined ? JSON.stringify(body) : undefined,
                signal: controller.signal,
            });
            clearTimeout(timeoutId);
            if (!resp.ok) {
                const text = await resp.text().catch(() => resp.statusText);
                throw new Error(`HipCortex ${method} ${path} → ${resp.status}: ${text}`);
            }
            return resp.json();
        }
        catch (err) {
            clearTimeout(timeoutId);
            throw err;
        }
    }
    async addMemory(req) {
        return this.request("POST", "/memory/add", req);
    }
    async queryMemory(params = {}) {
        return this.request("GET", "/memory/query", undefined, {
            actor: params.actor,
            action: params.action,
            record_type: params.record_type,
            limit: params.limit ?? 100,
            ...(params.include_expired ? { include_expired: "true" } : {}),
        });
    }
    async search(req) {
        return this.request("POST", "/memory/search", req);
    }
    async bulkAdd(req) {
        return this.request("POST", "/memory/bulk", req);
    }
    async forget(actor) {
        return this.request("DELETE", `/memory/forget/${actor}`);
    }
    async health() {
        try {
            await this.request("GET", "/health");
            return true;
        }
        catch {
            return false;
        }
    }
    async stats() {
        return this.request("GET", "/stats");
    }
    async coherenceStatus() {
        return this.request("GET", "/coherence/status");
    }
    /** GET /memory/live_beliefs — unified symbolic facts, hypotheses, world state, intel. */
    async liveBeliefs(params) {
        return this.request("GET", "/memory/live_beliefs", undefined, {
            actor: params?.actor,
            limit: params?.limit,
        });
    }
    /** POST /memory/reflect — AureusBridge CoT / hypothesis sample over memory context. */
    async reflect(req) {
        const body = typeof req === "string" ? { query: req } : req;
        return this.request("POST", "/memory/reflect", body);
    }
    /** POST /worldmodel/predict — single-step P(s'|s,a) world-model prediction. */
    async predict(req) {
        return this.request("POST", "/worldmodel/predict", req);
    }
    async addHumanMessage(sessionId, content) {
        return this.addMemory({ actor: sessionId, action: "human_message", target: content, record_type: "Temporal" });
    }
    async addAiMessage(sessionId, content) {
        return this.addMemory({ actor: sessionId, action: "ai_message", target: content, record_type: "Reflexion" });
    }
    async getConversationHistory(sessionId, limit = 50) {
        return this.queryMemory({ actor: sessionId, limit });
    }
    // ---------------------------------------------------------------------------
    // TMF — Tiered Memory Foundation graph methods (v0.4.0)
    // ---------------------------------------------------------------------------
    /** Create a directed graph edge between two memory records via CausalTopoGraph. */
    async linkMemories(req) {
        return this.request("POST", "/memory/link", {
            source_id: req.source_id,
            target_id: req.target_id,
            relation: req.relation ?? "related",
        });
    }
    /** Return neighboring memory records linked via the CausalTopoGraph. */
    async getNeighbors(recordId, limit = 10) {
        return this.request("GET", `/memory/neighbors/${recordId}`, undefined, {
            limit,
        });
    }
    /**
     * PPR-ranked related memory search seeded from a given record ID.
     * Uses Personalized PageRank (α=0.85, 20 rounds) over the CausalTopoGraph.
     */
    async searchRelated(seedId, limit = 10) {
        return this.request("GET", "/memory/search/related", undefined, {
            seed_id: seedId,
            limit,
        });
    }
    /** Delete a single memory record by UUID. */
    async deleteMemory(recordId) {
        return this.request("DELETE", `/memory/${recordId}`);
    }
    /** Evaluate capability gates and self-model execution readiness. */
    async canExecute(action = "rollout") {
        try {
            return await this.health();
        }
        catch {
            return false;
        }
    }
    /** Predict next state trajectory via multi-step Monte Carlo Tree Search (/worldmodel/rollout). */
    async rollout(req) {
        return this.request("POST", "/worldmodel/rollout", req);
    }
}
exports.HipCortexClient = HipCortexClient;
//# sourceMappingURL=client.js.map