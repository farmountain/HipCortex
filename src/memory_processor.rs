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

    #[test]
    fn deduplicate_removes_duplicates() {
        let rec = MemoryRecord::new(
            crate::memory_record::MemoryType::Temporal,
            "a".into(),
            "b".into(),
            "c".into(),
            serde_json::json!({}),
        );
        let mut vec = vec![rec.clone(), rec];
        MemoryProcessor::deduplicate(&mut vec);
        assert_eq!(vec.len(), 1);
    }
}
