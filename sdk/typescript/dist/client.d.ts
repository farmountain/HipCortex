import type { AddMemoryRequest, AddMemoryResponse, BulkAddRequest, BulkAddResponse, CoherenceStatusResponse, ForgetResponse, HipCortexClientOptions, QueryMemoryResponse, QueryParams, SearchRequest, SearchResponse, StatsResponse, LinkMemoriesRequest, LinkMemoriesResponse, NeighborsResponse, RelatedSearchResponse, DeleteMemoryResponse, RolloutRequest, RolloutResponse, LiveBeliefsResponse, ReflectRequest, ReflectResponse, PredictRequest, PredictResponse } from "./types";
export declare class HipCortexClient {
    private readonly baseUrl;
    private readonly headers;
    private readonly timeout;
    constructor(options?: HipCortexClientOptions);
    private request;
    addMemory(req: AddMemoryRequest): Promise<AddMemoryResponse>;
    queryMemory(params?: QueryParams): Promise<QueryMemoryResponse>;
    search(req: SearchRequest): Promise<SearchResponse>;
    bulkAdd(req: BulkAddRequest): Promise<BulkAddResponse>;
    forget(actor: string): Promise<ForgetResponse>;
    health(): Promise<boolean>;
    stats(): Promise<StatsResponse>;
    coherenceStatus(): Promise<CoherenceStatusResponse>;
    /** GET /memory/live_beliefs — unified symbolic facts, hypotheses, world state, intel. */
    liveBeliefs(params?: {
        actor?: string;
        limit?: number;
    }): Promise<LiveBeliefsResponse>;
    /** POST /memory/reflect — AureusBridge CoT / hypothesis sample over memory context. */
    reflect(req: ReflectRequest | string): Promise<ReflectResponse>;
    /** POST /worldmodel/predict — single-step P(s'|s,a) world-model prediction. */
    predict(req: PredictRequest): Promise<PredictResponse>;
    addHumanMessage(sessionId: string, content: string): Promise<AddMemoryResponse>;
    addAiMessage(sessionId: string, content: string): Promise<AddMemoryResponse>;
    getConversationHistory(sessionId: string, limit?: number): Promise<QueryMemoryResponse>;
    /** Create a directed graph edge between two memory records via CausalTopoGraph. */
    linkMemories(req: LinkMemoriesRequest): Promise<LinkMemoriesResponse>;
    /** Return neighboring memory records linked via the CausalTopoGraph. */
    getNeighbors(recordId: string, limit?: number): Promise<NeighborsResponse>;
    /**
     * PPR-ranked related memory search seeded from a given record ID.
     * Uses Personalized PageRank (α=0.85, 20 rounds) over the CausalTopoGraph.
     */
    searchRelated(seedId: string, limit?: number): Promise<RelatedSearchResponse>;
    /** Delete a single memory record by UUID. */
    deleteMemory(recordId: string): Promise<DeleteMemoryResponse>;
    /** Evaluate capability gates and self-model execution readiness. */
    canExecute(action?: string): Promise<boolean>;
    /** Predict next state trajectory via multi-step Monte Carlo Tree Search (/worldmodel/rollout). */
    rollout(req: RolloutRequest): Promise<RolloutResponse>;
}
//# sourceMappingURL=client.d.ts.map