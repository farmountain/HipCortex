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
