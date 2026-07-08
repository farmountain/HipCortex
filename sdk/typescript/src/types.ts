export interface AddMemoryRequest {
  actor: string;
  action: string;
  target: string;
  record_type?: "Temporal" | "Symbolic" | "Procedural" | "Reflexion" | "Perception" |
    "Episodic" | "Semantic" | "LongTerm" | "ShortTerm" | "Working" | "Reflexive" | "Perceptual";
  metadata?: Record<string, unknown>;
  ttl_seconds?: number;
  /** Reliability signal [0.0, 1.0]. Acts as a permanent score ceiling in search ranking. Default 1.0. */
  confidence?: number;
  /** Source identifier for trust weighting (e.g. "user-input", "claude-3-7"). */
  source?: string;
  /** Per-record decay rate multiplier [0.0, 2.0]. 1.0=normal, 0=no decay, >1=faster. Default 1.0. */
  decay_factor?: number;
  /** Per-record decay half-life in seconds. Default 2592000 (30 days). */
  decay_half_life_secs?: number;
  /** Categorization tags for filtering and RAG retrieval (e.g. ["bug", "decision"]). */
  tags?: string[];
  /** Search ranking priority. "pinned" records always surface first at score 2.0 in semantic search. */
  priority?: "pinned" | "high" | "normal" | "low";
}

export interface MemoryRecord {
  id: string;
  record_type: string;
  timestamp: string;
  actor: string;
  action: string;
  target: string;
  metadata: Record<string, unknown>;
  integrity: string | null;
}

export interface AddMemoryResponse {
  success: boolean;
  record_id: string | null;
  error: string | null;
}

export interface QueryParams {
  actor?: string;
  action?: string;
  record_type?: string;
  limit?: number;
  include_expired?: boolean;
}

export interface QueryMemoryResponse {
  records: MemoryRecord[];
  total: number;
}

export interface SearchRequest {
  query: string;
  embedding?: number[];
  limit?: number;
}

export interface SearchResult {
  score: number;
  record: MemoryRecord;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
}

export interface BulkAddRequest {
  records: AddMemoryRequest[];
}

export interface BulkAddResponse {
  success: boolean;
  inserted: number;
  failed: number;
  record_ids: string[];
  errors: string[];
}

export interface ForgetResponse {
  success: boolean;
  actor: string;
  records_deleted: number;
  symbolic_nodes_deleted: number;
  error: string | null;
}

export interface StatsResponse {
  total_records: number;
  active_records: number;
  by_type: Record<string, number>;
  unique_actors: number;
  metering_enabled: boolean;
  tier_counts: Record<string, number>;
}

export interface CoherenceStatusResponse {
  coherence_score: number;
  total_checks: number;
  inconsistencies_found: number;
  healthy: boolean;
}

export interface HipCortexClientOptions {
  baseUrl?: string;
  apiKey?: string;
  timeout?: number;
}

// ---------------------------------------------------------------------------
// TMF — Tiered Memory Foundation graph types (v0.4.0)
// ---------------------------------------------------------------------------

export interface LinkMemoriesRequest {
  source_id: string;
  target_id: string;
  /** Edge label (default: "related") */
  relation?: string;
}

export interface LinkMemoriesResponse {
  success: boolean;
  source_id: string;
  target_id: string;
  relation: string;
}

export interface NeighborsResponse {
  records: MemoryRecord[];
  total: number;
}

export interface RelatedSearchParams {
  seed_id: string;
  limit?: number;
}

export interface RelatedSearchResponse {
  results: SearchResult[];
}

export interface DeleteMemoryResponse {
  success: boolean;
  record_id: string;
}
