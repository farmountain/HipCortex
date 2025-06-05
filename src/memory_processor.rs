use std::collections::HashSet;

use crate::memory_record::MemoryRecord;

pub struct MemoryProcessor;

impl MemoryProcessor {
    pub fn deduplicate(records: &mut Vec<MemoryRecord>) {
        let mut seen = HashSet::new();
        records.retain(|r| seen.insert(r.id));
    }
}
