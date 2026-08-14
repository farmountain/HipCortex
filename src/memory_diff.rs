use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::memory_record::MemoryRecord;

pub fn diff_snapshots(
    a: &[MemoryRecord],
    b: &[MemoryRecord],
) -> (Vec<MemoryRecord>, Vec<MemoryRecord>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let ids_a: std::collections::HashSet<_> = a.iter().map(|r| r.id).collect();
    let ids_b: std::collections::HashSet<_> = b.iter().map(|r| r.id).collect();
    for rec in b {
        if !ids_a.contains(&rec.id) {
            added.push(rec.clone());
        }
    }
    for rec in a {
        if !ids_b.contains(&rec.id) {
            removed.push(rec.clone());
        }
    }
    (added, removed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub record_id: Uuid,
    pub from_version: u32,
    pub to_version: u32,
    pub field_changes: Vec<FieldChange>,
    pub confidence_delta: f32,
    pub status_change: Option<(String, String)>,
    pub react_iterations_delta: u32,
}

/// Compute a structural diff between two snapshots of the same record.
pub fn compute_diff(from: &MemoryRecord, to: &MemoryRecord) -> StateDiff {
    let mut changes = Vec::new();

    macro_rules! check_field {
        ($field:ident) => {
            if from.$field != to.$field {
                changes.push(FieldChange {
                    field: stringify!($field).to_string(),
                    old_value: serde_json::to_value(&from.$field).unwrap_or(serde_json::Value::Null),
                    new_value: serde_json::to_value(&to.$field).unwrap_or(serde_json::Value::Null),
                });
            }
        };
    }

    check_field!(actor);
    check_field!(action);
    check_field!(target);
    check_field!(record_type);
    check_field!(tags);
    check_field!(priority);
    check_field!(source);

    StateDiff {
        record_id: from.id,
        from_version: from.version,
        to_version: to.version,
        field_changes: changes,
        confidence_delta: to.confidence - from.confidence,
        status_change: if from.status != to.status {
            Some((from.status.clone(), to.status.clone()))
        } else {
            None
        },
        react_iterations_delta: to.react_iteration.unwrap_or(0)
            .saturating_sub(from.react_iteration.unwrap_or(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_record::{MemoryRecord, MemoryType};

    #[test]
    fn diff_basic() {
        let r1 = MemoryRecord::new(
            MemoryType::Symbolic,
            "a".into(),
            "b".into(),
            "c".into(),
            serde_json::json!({}),
        );
        let r2 = MemoryRecord::new(
            MemoryType::Symbolic,
            "x".into(),
            "y".into(),
            "z".into(),
            serde_json::json!({}),
        );
        let set1 = vec![r1.clone()];
        let set2 = vec![r1, r2.clone()];
        let (added, removed) = diff_snapshots(&set1, &set2);
        assert_eq!(added.len(), 1);
        assert_eq!(removed.len(), 0);
        assert_eq!(added[0].id, r2.id);
    }
}
