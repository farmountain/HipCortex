// @generated automatically by Diesel CLI.
#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
use diesel::{allow_tables_to_appear_in_same_query, table};

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    symbolic_nodes (id) {
        id -> Uuid,
        labels -> Jsonb,
        weight -> Float4,
        metadata -> Nullable<Jsonb>,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    symbolic_edges (id) {
        id -> Integer,
        source -> Uuid,
        target -> Uuid,
        relation -> Text,
        confidence -> Float4,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    temporal_events (id) {
        id -> Uuid,
        timestamp -> Timestamp,
        content -> Text,
        agent_tags -> Nullable<Jsonb>,
        trigger_type -> Nullable<Text>,
        weight -> Float4,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    procedural_policies (id) {
        id -> Uuid,
        current_state -> Text,
        trigger_condition -> Nullable<Text>,
        effect -> Nullable<Text>,
        next_state -> Nullable<Text>,
        version -> Integer,
        reward -> Nullable<Float4>,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    perception_inputs (id) {
        id -> Uuid,
        timestamp -> Timestamp,
        source -> Nullable<Text>,
        modality -> Text,
        data -> Nullable<Jsonb>,
        meaning -> Nullable<Text>,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
table! {
    reflexion_snapshots (id) {
        id -> Uuid,
        input_trace -> Nullable<Uuid>,
        thoughts -> Nullable<Jsonb>,
        outcome -> Nullable<Text>,
        feedback -> Nullable<Float4>,
    }
}

#[cfg(any(feature = "postgres_backend", feature = "sqlite_backend"))]
allow_tables_to_appear_in_same_query!(
    symbolic_nodes,
    symbolic_edges,
    temporal_events,
    procedural_policies,
    perception_inputs,
    reflexion_snapshots,
);
