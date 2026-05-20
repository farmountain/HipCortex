import type {
  AddMemoryRequest, AddMemoryResponse,
  BulkAddRequest, BulkAddResponse,
  CoherenceStatusResponse, ForgetResponse,
  HipCortexClientOptions, QueryMemoryResponse,
  QueryParams, SearchRequest, SearchResponse,
  StatsResponse,
} from "./types";

export class HipCortexClient {
  private readonly baseUrl: string;
  private readonly headers: Record<string, string>;

  constructor(options: HipCortexClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? "http://localhost:3030").replace(/\/$/, "");
    this.headers = { "Content-Type": "application/json" };
    if (options.apiKey) this.headers["X-Api-Key"] = options.apiKey;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    params?: Record<string, string | number | undefined>,
  ): Promise<T> {
    let url = `${this.baseUrl}${path}`;
    if (params) {
      const entries = Object.entries(params).filter(([, v]) => v !== undefined && v !== null);
      if (entries.length > 0) {
        url += "?" + new URLSearchParams(entries.map(([k, v]) => [k, String(v)] as [string, string])).toString();
      }
    }
    const resp = await fetch(url, {
      method,
      headers: this.headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    if (!resp.ok) {
      const text = await resp.text().catch(() => resp.statusText);
      throw new Error(`HipCortex ${method} ${path} → ${resp.status}: ${text}`);
    }
    return resp.json() as Promise<T>;
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

  async addHumanMessage(sessionId: string, content: string): Promise<AddMemoryResponse> {
    return this.addMemory({ actor: sessionId, action: "human_message", target: content, record_type: "Temporal" });
  }

  async addAiMessage(sessionId: string, content: string): Promise<AddMemoryResponse> {
    return this.addMemory({ actor: sessionId, action: "ai_message", target: content, record_type: "Reflexion" });
  }

  async getConversationHistory(sessionId: string, limit = 50): Promise<QueryMemoryResponse> {
    return this.queryMemory({ actor: sessionId, limit });
  }
}
