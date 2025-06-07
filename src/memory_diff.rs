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
