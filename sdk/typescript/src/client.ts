import type {
  AddMemoryRequest, AddMemoryResponse,
  BulkAddRequest, BulkAddResponse,
  CoherenceStatusResponse, ForgetResponse,
  HipCortexClientOptions, QueryMemoryResponse,
  QueryParams, SearchRequest, SearchResponse,
  StatsResponse,
  LinkMemoriesRequest, LinkMemoriesResponse,
  NeighborsResponse, RelatedSearchResponse,
  DeleteMemoryResponse, RolloutRequest, RolloutResponse,
  LiveBeliefsResponse, ReflectRequest, ReflectResponse,
  PredictRequest, PredictResponse,
  CognitiveDelta, TransactResponse, TxStateDiff, SelfHealthResponse,
  CognitiveSnapshot, ForkCreateResponse, ForkStepResponse, RolloutResult,
} from "./types";

export class HipCortexClient {
  private readonly baseUrl: string;
  private readonly headers: Record<string, string>;
  private readonly timeout: number;

  constructor(options: HipCortexClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? "http://localhost:3030").replace(/\/$/, "");
    this.headers = { "Content-Type": "application/json" };
    if (options.apiKey) this.headers["X-Api-Key"] = options.apiKey;
    this.timeout = options.timeout ?? 10_000;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    params?: Record<string, string | number | boolean | undefined>,
  ): Promise<T> {
    let url = `${this.baseUrl}${path}`;
    if (params) {
      const entries = Object.entries(params).filter(([, v]) => v !== undefined && v !== null);
      if (entries.length > 0) {
        url += "?" + new URLSearchParams(entries.map(([k, v]) => [k, String(v)] as [string, string])).toString();
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
      return resp.json() as Promise<T>;
    } catch (err) {
      clearTimeout(timeoutId);
      throw err;
    }
  }

  async addMemory(req: AddMemoryRequest): Promise<AddMemoryResponse> {
    return this.request<AddMemoryResponse>("POST", "/memory/add", req);
  }

  async queryMemory(params: QueryParams = {}): Promise<QueryMemoryResponse> {
    return this.request<QueryMemoryResponse>("GET", "/memory/query", undefined, {
      actor: params.actor,
      action: params.action,
      record_type: params.record_type,
      limit: params.limit ?? 100,
      ...(params.include_expired ? { include_expired: "true" } : {}),
    });
  }

  async search(req: SearchRequest): Promise<SearchResponse> {
    return this.request<SearchResponse>("POST", "/memory/search", req);
  }

  async bulkAdd(req: BulkAddRequest): Promise<BulkAddResponse> {
    return this.request<BulkAddResponse>("POST", "/memory/bulk", req);
  }

  async forget(actor: string): Promise<ForgetResponse> {
    return this.request<ForgetResponse>("DELETE", `/memory/forget/${actor}`);
  }

  async health(): Promise<boolean> {
    try {
      await this.request<string>("GET", "/health");
      return true;
    } catch {
      return false;
    }
  }

  async stats(): Promise<StatsResponse> {
    return this.request<StatsResponse>("GET", "/stats");
  }

  async coherenceStatus(): Promise<CoherenceStatusResponse> {
    return this.request<CoherenceStatusResponse>("GET", "/coherence/status");
  }

  /** GET /memory/live_beliefs — unified symbolic facts, hypotheses, world state, intel. */
  async liveBeliefs(params?: { actor?: string; limit?: number }): Promise<LiveBeliefsResponse> {
    return this.request<LiveBeliefsResponse>("GET", "/memory/live_beliefs", undefined, {
      actor: params?.actor,
      limit: params?.limit,
    });
  }

  /** POST /memory/reflect — AureusBridge CoT / hypothesis sample over memory context. */
  async reflect(req: ReflectRequest | string): Promise<ReflectResponse> {
    const body: ReflectRequest = typeof req === "string" ? { query: req } : req;
    return this.request<ReflectResponse>("POST", "/memory/reflect", body);
  }

  /** POST /worldmodel/predict — single-step P(s'|s,a) world-model prediction. */
  async predict(req: PredictRequest): Promise<PredictResponse> {
    return this.request<PredictResponse>("POST", "/worldmodel/predict", req);
  }

  async addHumanMessage(sessionId: string, content: string): Promise<AddMemoryResponse> {
    return this.addMemory({ actor: sessionId, action: "human_message", target: content, record_type: "Temporal" });
  }

  async addAiMessage(sessionId: string, content: string): Promise<AddMemoryResponse> {
    return this.addMemory({ actor: sessionId, action: "ai_message", target: content, record_type: "Reflexion" });
  }

  async getConversationHistory(sessionId: string, limit = 50): Promise<QueryMemoryResponse> {
    return this.queryMemory({ actor: sessionId, limit });
  }

  // ---------------------------------------------------------------------------
  // TMF — Tiered Memory Foundation graph methods (v0.4.0)
  // ---------------------------------------------------------------------------

  /** Create a directed graph edge between two memory records via CausalTopoGraph. */
  async linkMemories(req: LinkMemoriesRequest): Promise<LinkMemoriesResponse> {
    return this.request<LinkMemoriesResponse>("POST", "/memory/link", {
      source_id: req.source_id,
      target_id: req.target_id,
      relation: req.relation ?? "related",
    });
  }

  /** Return neighboring memory records linked via the CausalTopoGraph. */
  async getNeighbors(recordId: string, limit = 10): Promise<NeighborsResponse> {
    return this.request<NeighborsResponse>("GET", `/memory/neighbors/${recordId}`, undefined, {
      limit,
    });
  }

  /**
   * PPR-ranked related memory search seeded from a given record ID.
   * Uses Personalized PageRank (α=0.85, 20 rounds) over the CausalTopoGraph.
   */
  async searchRelated(seedId: string, limit = 10): Promise<RelatedSearchResponse> {
    return this.request<RelatedSearchResponse>("GET", "/memory/search/related", undefined, {
      seed_id: seedId,
      limit,
    });
  }

  /** Delete a single memory record by UUID. */
  async deleteMemory(recordId: string): Promise<DeleteMemoryResponse> {
    return this.request<DeleteMemoryResponse>("DELETE", `/memory/${recordId}`);
  }

  /** Evaluate capability gates and self-model execution readiness. */
  async canExecute(action = "rollout"): Promise<boolean> {
    try {
      return await this.health();
    } catch {
      return false;
    }
  }

  /** Predict next state trajectory via multi-step Monte Carlo Tree Search (/worldmodel/rollout). */
  async rollout(req: RolloutRequest): Promise<RolloutResponse> {
    return this.request<RolloutResponse>("POST", "/worldmodel/rollout", req);
  }

  // ---------------------------------------------------------------------------
  // Phase 5: Agent Surfaces (v0.8.0)
  // ---------------------------------------------------------------------------

  static readonly VERSION = "0.8.0";

  async transact(delta: CognitiveDelta, actor: string): Promise<TransactResponse> {
    return this.request<TransactResponse>("POST", "/v1/cognitive/transact", { delta, actor });
  }

  async cognitiveDiff(fromTx: number, toTx: number): Promise<TxStateDiff> {
    return this.request<TxStateDiff>("GET", "/v1/cognitive/diff", undefined, { from_tx: fromTx, to_tx: toTx });
  }

  async selfHealth(): Promise<SelfHealthResponse> {
    return this.request<SelfHealthResponse>("GET", "/v1/self/health");
  }

  async cognitiveSnapshot(actor = ""): Promise<CognitiveSnapshot> {
    return this.request<CognitiveSnapshot>("GET", "/v1/cognitive/snapshot", undefined, { actor });
  }

  async forkCreate(): Promise<ForkCreateResponse> {
    return this.request<ForkCreateResponse>("POST", "/v1/fork", {});
  }

  async forkStep(forkId: string, action: string): Promise<ForkStepResponse> {
    return this.request<ForkStepResponse>("POST", `/v1/fork/${forkId}/step`, { action });
  }

  async forkSnapshot(forkId: string, actor = ""): Promise<CognitiveSnapshot> {
    return this.request<CognitiveSnapshot>("GET", `/v1/fork/${forkId}/snapshot`, undefined, { actor });
  }

  async forkDelete(forkId: string): Promise<{ ok: boolean }> {
    return this.request<{ ok: boolean }>("DELETE", `/v1/fork/${forkId}`);
  }

  async forkRollout(forkId: string, actions: string[], sigma2Max = 0.25): Promise<RolloutResult> {
    return this.request<RolloutResult>("POST", `/v1/fork/${forkId}/rollout`, { actions, sigma2_max: sigma2Max });
  }

  async consolidate(sourceIds: string[], summary: Record<string, unknown>): Promise<TransactResponse> {
    return this.transact({ type: "Consolidate", source_ids: sourceIds, summary }, "ts-sdk");
  }

  async forgetActor(actorId: string): Promise<TransactResponse> {
    return this.transact({ type: "ForgetActor", actor: actorId }, "ts-sdk");
  }

  async archiveRecord(id: string): Promise<TransactResponse> {
    return this.transact({ type: "ArchiveRecord", id }, "ts-sdk");
  }
}

