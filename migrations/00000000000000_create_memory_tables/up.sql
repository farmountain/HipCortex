CREATE TABLE symbolic_nodes (
    id UUID PRIMARY KEY,
    labels JSON NOT NULL,
    weight REAL DEFAULT 1.0,
    metadata JSON
);

CREATE TABLE symbolic_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source UUID NOT NULL REFERENCES symbolic_nodes(id),
    target UUID NOT NULL REFERENCES symbolic_nodes(id),
    relation TEXT NOT NULL,
    confidence REAL DEFAULT 1.0
);

CREATE INDEX idx_symbolic_edges_source ON symbolic_edges(source);
CREATE INDEX idx_symbolic_edges_target ON symbolic_edges(target);

CREATE TABLE temporal_events (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL,
    content TEXT NOT NULL,
    agent_tags JSON,
    trigger_type TEXT,
    weight REAL DEFAULT 1.0
);
CREATE INDEX idx_temporal_events_ts ON temporal_events(timestamp);

CREATE TABLE procedural_policies (
    id UUID PRIMARY KEY,
    current_state TEXT NOT NULL,
    trigger_condition TEXT,
    effect TEXT,
    next_state TEXT,
    version INTEGER,
    reward REAL
);

CREATE TABLE perception_inputs (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL,
    source TEXT,
    modality TEXT NOT NULL,
    data JSON,
    meaning TEXT
);

CREATE TABLE reflexion_snapshots (
    id UUID PRIMARY KEY,
    input_trace UUID,
    thoughts JSON,
    outcome TEXT,
    feedback REAL
);
