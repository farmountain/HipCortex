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
}
