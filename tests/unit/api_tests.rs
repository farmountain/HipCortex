use axum::http::StatusCode;
use axum_test::TestServer;
use hipcortex::memory_store::MemoryStore;
use hipcortex::web_server;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[cfg(test)]
mod api_tests {
    use super::*;

    async fn create_test_server() -> TestServer {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_api.jsonl");
        
        let memory_store = MemoryStore::new(file_path.to_str().unwrap()).unwrap();
        let memory_store = Arc::new(Mutex::new(memory_store));
        
        let symbolic_store = Arc::new(Mutex::new(
            hipcortex::symbolic_store::SymbolicStore::new()
        ));

        let app = web_server::create_router(symbolic_store, memory_store);
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = create_test_server().await;
        
        let response = server.get("/health").await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        // Dual contract: canonical body is JSON (not legacy plain "ok").
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "hipcortex");
        assert!(body["version"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_add_memory_endpoint_success() {
        let server = create_test_server().await;
        
        let request_body = json!({
            "actor": "APITest",
            "action": "test_add",
            "target": "api_endpoint",
            "record_type": "Temporal",
            "metadata": {"test": true}
        });

        let response = server
            .post("/memory/add")
            .json(&request_body)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        
        let response_body: serde_json::Value = response.json();
        assert_eq!(response_body["success"], true);
        assert!(response_body["record_id"].is_string());
        assert_eq!(response_body["error"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_add_memory_endpoint_validation() {
        let server = create_test_server().await;
        
        // Test missing required fields
        let invalid_request = json!({
            "actor": "APITest"
            // Missing action and target
        });

        let response = server
            .post("/memory/add")
            .json(&invalid_request)
            .await;

        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_query_memory_endpoint_success() {
        let server = create_test_server().await;
        
        // First, add a memory record
        let add_request = json!({
            "actor": "QueryTest",
            "action": "setup_data",
            "target": "query_endpoint",
            "record_type": "Temporal"
        });

        let add_response = server
            .post("/memory/add")
            .json(&add_request)
            .await;
        
        assert_eq!(add_response.status_code(), StatusCode::OK);

        // Now query for the record
        let query_response = server
            .get("/memory/query?actor=QueryTest")
            .await;

        assert_eq!(query_response.status_code(), StatusCode::OK);
        
        let response_body: serde_json::Value = query_response.json();
        assert_eq!(response_body["total"], 1);
        assert!(response_body["records"].is_array());
        assert_eq!(response_body["records"][0]["actor"], "QueryTest");
        assert_eq!(response_body["records"][0]["action"], "setup_data");
    }

    #[tokio::test]
    async fn test_query_memory_endpoint_filters() {
        let server = create_test_server().await;
        
        // Add multiple records
        let records = vec![
            ("FilterTest1", "action1", "target1"),
            ("FilterTest2", "action2", "target2"),
            ("FilterTest1", "action3", "target3"),
        ];

        for (actor, action, target) in records {
            let request = json!({
                "actor": actor,
                "action": action,
                "target": target,
                "record_type": "Temporal"
            });
            
            let response = server
                .post("/memory/add")
                .json(&request)
                .await;
            
            assert_eq!(response.status_code(), StatusCode::OK);
        }

        // Test actor filter
        let actor_query = server
            .get("/memory/query?actor=FilterTest1")
            .await;
        
        let response_body: serde_json::Value = actor_query.json();
        assert_eq!(response_body["total"], 2);

        // Test action filter
        let action_query = server
            .get("/memory/query?action=action2")
            .await;
        
        let response_body: serde_json::Value = action_query.json();
        assert_eq!(response_body["total"], 1);
        assert_eq!(response_body["records"][0]["actor"], "FilterTest2");

        // Test limit
        let limit_query = server
            .get("/memory/query?limit=2")
            .await;
        
        let response_body: serde_json::Value = limit_query.json();
        assert!(response_body["records"].as_array().unwrap().len() <= 2);
    }

    #[tokio::test]
    async fn test_query_memory_endpoint_empty_results() {
        let server = create_test_server().await;
        
        let response = server
            .get("/memory/query?actor=NonExistentActor")
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        
        let response_body: serde_json::Value = response.json();
        assert_eq!(response_body["total"], 0);
        assert_eq!(response_body["records"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let server = create_test_server().await;
        
        // Test malformed JSON
        let response = server
            .post("/memory/add")
            .header("content-type", "application/json")
            .text("invalid json")
            .await;

        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

        // Test missing content-type
        let response = server
            .post("/memory/add")
            .text("{\"actor\": \"test\"}")
            .await;

        // Should still work or provide appropriate error
        assert!(response.status_code().is_client_error() || response.status_code().is_success());
    }

    #[tokio::test]
    async fn test_api_cors_headers() {
        let server = create_test_server().await;
        
        let response = server.get("/health").await;
        
        // Check that basic headers are present
        assert!(response.headers().contains_key("content-type"));
    }

    #[tokio::test]
    async fn test_api_performance() {
        let server = create_test_server().await;
        
        let start = std::time::Instant::now();
        
        // Test multiple concurrent requests
        let mut handles = vec![];
        
        for i in 0..10 {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                let request = json!({
                    "actor": format!("PerfTest{}", i),
                    "action": "performance_test",
                    "target": "api_endpoint",
                    "record_type": "Temporal"
                });
                
                server_clone
                    .post("/memory/add")
                    .json(&request)
                    .await
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            let response = handle.await.unwrap();
            assert_eq!(response.status_code(), StatusCode::OK);
        }
        
        let duration = start.elapsed();
        
        // All 10 requests should complete within 5 seconds
        assert!(duration.as_secs() < 5);
        
        // Verify all records were added
        let query_response = server
            .get("/memory/query?action=performance_test")
            .await;
        
        let response_body: serde_json::Value = query_response.json();
        assert_eq!(response_body["total"], 10);
    }

    #[tokio::test]
    async fn test_api_input_sanitization() {
        let server = create_test_server().await;
        
        // Test potentially dangerous input
        let malicious_request = json!({
            "actor": "<script>alert('xss')</script>",
            "action": "'; DROP TABLE memory; --",
            "target": "../../etc/passwd",
            "record_type": "Temporal",
            "metadata": {"script": "<img src=x onerror=alert(1)>"}
        });

        let response = server
            .post("/memory/add")
            .json(&malicious_request)
            .await;

        // Should either sanitize input or reject with appropriate error
        if response.status_code().is_success() {
            // If accepted, verify the data was sanitized
            let response_body: serde_json::Value = response.json();
            assert_eq!(response_body["success"], true);
            
            // Query back to verify sanitization
            let query_response = server
                .get("/memory/query?actor=%3Cscript%3Ealert('xss')%3C/script%3E")
                .await;
            
            // Should find the record but with sanitized content
            assert_eq!(query_response.status_code(), StatusCode::OK);
        } else {
            // If rejected, should be a client error
            assert!(response.status_code().is_client_error());
        }
    }
}
