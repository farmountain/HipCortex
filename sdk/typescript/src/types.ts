export interface AddMemoryRequest {
  actor: string;
  action: string;
  target: string;
  record_type?: "Temporal" | "Symbolic" | "Procedural" | "Reflexion" | "Perception";
  metadata?: Record<string, unknown>;
  ttl_seconds?: number;
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
