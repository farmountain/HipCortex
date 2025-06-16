use chrono::{DateTime, Utc};

use crate::memory_record::{MemoryRecord, MemoryType};

pub struct MemoryQuery;

impl MemoryQuery {
    pub fn by_type<'a>(records: &'a [MemoryRecord], t: MemoryType) -> Vec<&'a MemoryRecord> {
        records.iter().filter(|r| r.record_type == t).collect()
    }

    pub fn by_actor<'a>(records: &'a [MemoryRecord], actor: &str) -> Vec<&'a MemoryRecord> {
        records.iter().filter(|r| r.actor == actor).collect()
    }

    pub fn by_action<'a>(records: &'a [MemoryRecord], action: &str) -> Vec<&'a MemoryRecord> {
        records.iter().filter(|r| r.action == action).collect()
    }

    pub fn by_target<'a>(records: &'a [MemoryRecord], target: &str) -> Vec<&'a MemoryRecord> {
        records.iter().filter(|r| r.target == target).collect()
    }

    /// Perform a case-insensitive substring search across actor, action and target fields.
    pub fn search<'a>(records: &'a [MemoryRecord], query: &str) -> Vec<&'a MemoryRecord> {
        let q = query.to_lowercase();
        records
            .iter()
            .filter(|r| {
                r.actor.to_lowercase().contains(&q)
                    || r.action.to_lowercase().contains(&q)
                    || r.target.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn since<'a>(records: &'a [MemoryRecord], ts: DateTime<Utc>) -> Vec<&'a MemoryRecord> {
        records.iter().filter(|r| r.timestamp >= ts).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_by_actor() {
        let rec = MemoryRecord::new(
            MemoryType::Temporal,
            "actor".into(),
            "do".into(),
            "thing".into(),
            serde_json::json!({}),
        );
        let vec = vec![rec];
        let results = MemoryQuery::by_actor(&vec, "actor");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_by_action_and_target() {
        let rec1 = MemoryRecord::new(
            MemoryType::Temporal,
            "actor".into(),
            "do".into(),
            "thing".into(),
            serde_json::json!({}),
        );
        let rec2 = MemoryRecord::new(
            MemoryType::Temporal,
            "actor2".into(),
            "play".into(),
            "ball".into(),
            serde_json::json!({}),
        );
        let vec = vec![rec1, rec2];
        assert_eq!(MemoryQuery::by_action(&vec, "play").len(), 1);
        assert_eq!(MemoryQuery::by_target(&vec, "thing").len(), 1);
    }
}
