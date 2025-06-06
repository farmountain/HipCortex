use std::collections::HashSet;

use crate::memory_record::MemoryRecord;

pub struct MemoryProcessor;

impl MemoryProcessor {
    pub fn deduplicate(records: &mut Vec<MemoryRecord>) {
        let mut seen = HashSet::new();
        records.retain(|r| seen.insert(r.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_record::MemoryType;

    #[test]
    fn dedup_removes_duplicates() {
        let mut recs = Vec::new();
        let rec = MemoryRecord { id: uuid::Uuid::new_v4(), record_type: MemoryType::Temporal, timestamp: chrono::Utc::now(), actor: "a".into(), action: "b".into(), target: "c".into(), metadata: serde_json::json!({}) };
        recs.push(rec.clone());
        recs.push(rec);
        MemoryProcessor::deduplicate(&mut recs);
        assert_eq!(recs.len(), 1);
    }
}
