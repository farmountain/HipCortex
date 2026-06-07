use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::FileBackend;
use serde_json::json;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_memory_record_creation() {
        let record = MemoryRecord::new(
            MemoryType::Temporal,
            "UnitTest".into(),
            "test_creation".into(),
            "memory_record".into(),
            json!({"test": true}),
        );

        assert_eq!(record.actor, "UnitTest");
        assert_eq!(record.action, "test_creation");
        assert_eq!(record.target, "memory_record");
        assert_eq!(record.record_type, MemoryType::Temporal);
    }

    #[test]
    fn test_memory_record_hash_computation() {
        let mut record = MemoryRecord::new(
            MemoryType::Temporal,
            "UnitTest".into(),
            "test_hash".into(),
            "integrity".into(),
            json!({}),
        );

        let hash = record.compute_hash();
        record.integrity = Some(hash.clone());

        assert!(record.integrity.is_some());
        assert_eq!(record.integrity.unwrap(), hash);
        assert!(hash.len() > 0);
    }

    #[test]
    fn test_memory_store_creation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_memory.jsonl");
        
        let store = MemoryStore::new(file_path.to_str().unwrap());
        assert!(store.is_ok());
    }

    #[test]
    fn test_memory_store_add_and_retrieve() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_add_retrieve.jsonl");
        
        let mut store = MemoryStore::new(file_path.to_str().unwrap()).unwrap();
        
        let record = MemoryRecord::new(
            MemoryType::Temporal,
            "UnitTest".into(),
            "test_add_retrieve".into(),
            "memory_store".into(),
            json!({"test_id": 1}),
        );

        let record_id = record.id;
        let add_result = store.add(record);
        assert!(add_result.is_ok());

        let all_records = store.all();
        assert_eq!(all_records.len(), 1);
        assert_eq!(all_records[0].id, record_id);
        assert_eq!(all_records[0].actor, "UnitTest");
    }

    #[test]
    fn test_memory_store_persistence() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_persistence.jsonl");
        
        // Add record to store
        {
            let mut store = MemoryStore::new(file_path.to_str().unwrap()).unwrap();
            let record = MemoryRecord::new(
                MemoryType::Temporal,
                "PersistenceTest".into(),
                "test_persistence".into(),
                "file_system".into(),
                json!({"persistent": true}),
            );
            store.add(record).unwrap();
        } // Store goes out of scope

        // Verify persistence by creating new store instance
        {
            let store = MemoryStore::new(file_path.to_str().unwrap()).unwrap();
            let all_records = store.all();
            assert_eq!(all_records.len(), 1);
            assert_eq!(all_records[0].actor, "PersistenceTest");
        }
    }

    #[test]
    fn test_memory_type_serialization() {
        let types = vec![
            MemoryType::Temporal,
            MemoryType::Symbolic,
            MemoryType::Procedural,
            MemoryType::Reflexion,
            MemoryType::Perception,
        ];

        for memory_type in types {
            let serialized = serde_json::to_string(&memory_type).unwrap();
            let deserialized: MemoryType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(memory_type, deserialized);
        }
    }

    #[test]
    fn test_memory_record_metadata_handling() {
        let record = MemoryRecord::new(
            MemoryType::Temporal,
            "MetadataTest".into(),
            "test_metadata".into(),
            "json_handling".into(),
            json!({
                "string_field": "test_value",
                "number_field": 42,
                "boolean_field": true,
                "array_field": [1, 2, 3],
                "object_field": {"nested": "value"}
            }),
        );

        // Test metadata access
        assert_eq!(record.metadata["string_field"], "test_value");
        assert_eq!(record.metadata["number_field"], 42);
        assert_eq!(record.metadata["boolean_field"], true);

        // Test serialization/deserialization
        let serialized = serde_json::to_string(&record).unwrap();
        let deserialized: MemoryRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(record.metadata, deserialized.metadata);
    }

    #[test]
    fn test_concurrent_memory_operations() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_concurrent.jsonl");
        
        let store = Arc::new(Mutex::new(
            MemoryStore::new(file_path.to_str().unwrap()).unwrap()
        ));

        let mut handles = vec![];

        // Spawn multiple threads to add records concurrently
        for i in 0..5 {
            let store_clone = Arc::clone(&store);
            let handle = thread::spawn(move || {
                let record = MemoryRecord::new(
                    MemoryType::Temporal,
                    format!("ConcurrentTest_{}", i),
                    "concurrent_add".into(),
                    "thread_safety".into(),
                    json!({"thread_id": i}),
                );

                let mut store = store_clone.lock().unwrap();
                store.add(record).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all records were added
        let store = store.lock().unwrap();
        let all_records = store.all();
        assert_eq!(all_records.len(), 5);
    }

    #[test]
    fn test_memory_record_validation() {
        // Test valid record
        let mut valid_record = MemoryRecord::new(
            MemoryType::Temporal,
            "ValidTest".into(),
            "valid_action".into(),
            "valid_target".into(),
            json!({}),
        );
        valid_record.integrity = None;

        assert!(!valid_record.actor.is_empty());
        assert!(!valid_record.action.is_empty());
        assert!(!valid_record.target.is_empty());

        // Test edge cases
        let mut edge_record = MemoryRecord::new(
            MemoryType::Temporal,
            "A".repeat(100),
            "action_with_special_chars_!@#$%".into(),
            "target-with-hyphens_and_underscores".into(),
            json!({"large_data": "x".repeat(1000)}),
        );
        edge_record.integrity = None;

        assert!(edge_record.actor.len() == 100);
        assert!(edge_record.action.contains("!@#$%"));
        assert!(edge_record.target.contains("-") && edge_record.target.contains("_"));
    }
}
