// @generated automatically by Diesel CLI.
use diesel::{allow_tables_to_appear_in_same_query, table};

table! {
    symbolic_nodes (id) {
        id -> Uuid,
        labels -> Jsonb,
        weight -> Float4,
        metadata -> Nullable<Jsonb>,
    }
}

table! {
    symbolic_edges (id) {
        id -> Integer,
        source -> Uuid,
        target -> Uuid,
        relation -> Text,
        confidence -> Float4,
    }
}

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

table! {
    reflexion_snapshots (id) {
        id -> Uuid,
        input_trace -> Nullable<Uuid>,
        thoughts -> Nullable<Jsonb>,
        outcome -> Nullable<Text>,
        feedback -> Nullable<Float4>,
    }
}

allow_tables_to_appear_in_same_query!(
    symbolic_nodes,
    symbolic_edges,
    temporal_events,
    procedural_policies,
    perception_inputs,
    reflexion_snapshots,
);
