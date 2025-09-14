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
        let record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "UnitTest".to_string(),
            action: "test_creation".to_string(),
            target: "memory_record".to_string(),
            metadata: json!({"test": true}),
            integrity: None,
        };

        assert_eq!(record.actor, "UnitTest");
        assert_eq!(record.action, "test_creation");
        assert_eq!(record.target, "memory_record");
        assert_eq!(record.record_type, MemoryType::Temporal);
    }

    #[test]
    fn test_memory_record_hash_computation() {
        let mut record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "UnitTest".to_string(),
            action: "test_hash".to_string(),
            target: "integrity".to_string(),
            metadata: json!({}),
            integrity: None,
        };

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
        
        let record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "UnitTest".to_string(),
            action: "test_add_retrieve".to_string(),
            target: "memory_store".to_string(),
            metadata: json!({"test_id": 1}),
            integrity: None,
        };

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
            let record = MemoryRecord {
                id: Uuid::new_v4(),
                record_type: MemoryType::Temporal,
                timestamp: chrono::Utc::now(),
                actor: "PersistenceTest".to_string(),
                action: "test_persistence".to_string(),
                target: "file_system".to_string(),
                metadata: json!({"persistent": true}),
                integrity: None,
            };
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
        let mut record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "MetadataTest".to_string(),
            action: "test_metadata".to_string(),
            target: "json_handling".to_string(),
            metadata: json!({
                "string_field": "test_value",
                "number_field": 42,
                "boolean_field": true,
                "array_field": [1, 2, 3],
                "object_field": {"nested": "value"}
            }),
            integrity: None,
        };

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
    fn test_error_handling_invalid_file_path() {
        let result = MemoryStore::new("/invalid/path/that/does/not/exist.jsonl");
        assert!(result.is_err());
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
                let record = MemoryRecord {
                    id: Uuid::new_v4(),
                    record_type: MemoryType::Temporal,
                    timestamp: chrono::Utc::now(),
                    actor: format!("ConcurrentTest_{}", i),
                    action: "concurrent_add".to_string(),
                    target: "thread_safety".to_string(),
                    metadata: json!({"thread_id": i}),
                    integrity: None,
                };

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
        let valid_record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "ValidTest".to_string(),
            action: "valid_action".to_string(),
            target: "valid_target".to_string(),
            metadata: json!({}),
            integrity: None,
        };

        assert!(!valid_record.actor.is_empty());
        assert!(!valid_record.action.is_empty());
        assert!(!valid_record.target.is_empty());

        // Test edge cases
        let edge_record = MemoryRecord {
            id: Uuid::new_v4(),
            record_type: MemoryType::Temporal,
            timestamp: chrono::Utc::now(),
            actor: "A".repeat(100), // Long actor name
            action: "action_with_special_chars_!@#$%".to_string(),
            target: "target-with-hyphens_and_underscores".to_string(),
            metadata: json!({"large_data": "x".repeat(1000)}),
            integrity: None,
        };

        assert!(edge_record.actor.len() == 100);
        assert!(edge_record.action.contains("!@#$%"));
        assert!(edge_record.target.contains("-") && edge_record.target.contains("_"));
    }
}
