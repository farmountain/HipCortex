use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use reqwest;
use serde_json::json;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use tokio;

struct TestServer {
    process: Option<Child>,
    base_url: String,
    _temp_dir: tempfile::TempDir,
}

impl TestServer {
    async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        let port = 10000 + (rand::random::<u16>() % 10000);
        let base_url = format!("http://127.0.0.1:{}", port);

        // Start the server in the background
        let mut process = Command::new("cargo")
            .args(&[
                "run",
                "--bin", "webserver",
                "--features", "web-server,petgraph_backend"
            ])
            .env("DATA_DIR", temp_dir.path().to_str().unwrap())
            .env("PORT", port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for server to start
        let client = reqwest::Client::new();
        
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(500));
            if let Ok(response) = client.get(&format!("{}/health", base_url)).send().await {
                if response.status().is_success() {
                    return Ok(TestServer {
                        process: Some(process),
                        base_url,
                        _temp_dir: temp_dir,
                    });
                }
            }
        }

        process.kill()?;
        Err("Server failed to start within timeout".into())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }

    async fn add_memory(&self, request: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/memory/add", self.base_url))
            .json(&request)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        let body = response.json::<serde_json::Value>().await?;
        Ok(body)
    }

    async fn query_memory(&self, params: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = if params.is_empty() {
            format!("{}/memory/query", self.base_url)
        } else {
            format!("{}/memory/query?{}", self.base_url, params)
        };
        
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        let body = response.json::<serde_json::Value>().await?;
        Ok(body)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

#[tokio::test]
async fn test_sit_server_startup_and_health() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    let health = server.health_check().await
        .expect("Failed to perform health check");
    
    assert!(health, "Server should be healthy");
}

#[tokio::test]
async fn test_sit_end_to_end_memory_operations() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    // Test 1: Add memory record
    let add_request = json!({
        "actor": "SIT_Test",
        "action": "integration_test",
        "target": "end_to_end_flow",
        "record_type": "Temporal",
        "metadata": {
            "test_type": "SIT",
            "timestamp": "2025-09-13T10:00:00Z"
        }
    });
    
    let add_response = server.add_memory(add_request).await
        .expect("Failed to add memory record");
    
    assert_eq!(add_response["success"], true);
    assert!(add_response["record_id"].is_string());
    
    let record_id = add_response["record_id"].as_str().unwrap();
    
    // Test 2: Query by actor
    let query_response = server.query_memory("actor=SIT_Test").await
        .expect("Failed to query memory");
    
    assert_eq!(query_response["total"], 1);
    assert_eq!(query_response["records"][0]["actor"], "SIT_Test");
    assert_eq!(query_response["records"][0]["action"], "integration_test");
    assert_eq!(query_response["records"][0]["id"], record_id);
    
    // Test 3: Query by action
    let action_query = server.query_memory("action=integration_test").await
        .expect("Failed to query by action");
    
    assert_eq!(action_query["total"], 1);
    assert_eq!(action_query["records"][0]["action"], "integration_test");
    
    // Test 4: Query with limit
    let limit_query = server.query_memory("limit=1").await
        .expect("Failed to query with limit");
    
    assert!(limit_query["records"].as_array().unwrap().len() <= 1);
}

#[tokio::test]
async fn test_sit_multiple_concurrent_operations() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    let mut handles = vec![];
    
    // Spawn 10 concurrent add operations
    for i in 0..10 {
        let server_url = server.base_url.clone();
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();
            let request = json!({
                "actor": format!("ConcurrentTest_{}", i),
                "action": "concurrent_operation",
                "target": "stress_test",
                "record_type": "Temporal",
                "metadata": {"thread_id": i}
            });
            
            let response = client
                .post(&format!("{}/memory/add", server_url))
                .json(&request)
                .timeout(Duration::from_secs(10))
                .send()
                .await?;
            
            let body = response.json::<serde_json::Value>().await?;
            Ok::<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>(body)
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        let response = result.expect("Request failed");
        assert_eq!(response["success"], true);
    }
    
    // Verify all records were added
    let query_response = server.query_memory("action=concurrent_operation").await
        .expect("Failed to query concurrent records");
    
    assert_eq!(query_response["total"], 10);
}

#[tokio::test]
async fn test_sit_error_handling_and_recovery() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    let client = reqwest::Client::new();
    
    // Test 1: Invalid JSON request
    let invalid_response = client
        .post(&format!("{}/memory/add", server.base_url))
        .header("content-type", "application/json")
        .body("invalid json")
        .send()
        .await
        .expect("Failed to send request");
    
    assert!(invalid_response.status().is_client_error());
    
    // Test 2: Missing required fields
    let incomplete_request = json!({
        "actor": "ErrorTest"
        // Missing action and target
    });
    
    let incomplete_response = client
        .post(&format!("{}/memory/add", server.base_url))
        .json(&incomplete_request)
        .send()
        .await
        .expect("Failed to send incomplete request");
    
    assert!(incomplete_response.status().is_client_error() || 
            incomplete_response.status().is_server_error());
    
    // Test 3: Server should still be functional after errors
    let health = server.health_check().await
        .expect("Health check failed");
    
    assert!(health, "Server should still be healthy after error conditions");
    
    // Test 4: Valid request should still work
    let valid_request = json!({
        "actor": "RecoveryTest",
        "action": "post_error_test",
        "target": "error_recovery",
        "record_type": "Temporal"
    });
    
    let valid_response = server.add_memory(valid_request).await
        .expect("Valid request failed after errors");
    
    assert_eq!(valid_response["success"], true);
}

#[tokio::test]
async fn test_sit_data_persistence_across_requests() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    // Add multiple records
    let records = vec![
        ("PersistenceTest1", "action1", "target1"),
        ("PersistenceTest2", "action2", "target2"),
        ("PersistenceTest3", "action3", "target3"),
    ];
    
    for (actor, action, target) in &records {
        let request = json!({
            "actor": actor,
            "action": action,
            "target": target,
            "record_type": "Temporal"
        });
        
        let response = server.add_memory(request).await
            .expect("Failed to add record");
        
        assert_eq!(response["success"], true);
    }
    
    // Query all records to verify persistence
    let query_response = server.query_memory("").await
        .expect("Failed to query all records");
    
    let total = query_response["total"].as_u64().unwrap();
    assert!(total >= 3, "Should have at least 3 records");
    
    // Verify specific records exist
    for (actor, action, target) in &records {
        let actor_query = server.query_memory(&format!("actor={}", actor)).await
            .expect("Failed to query by actor");
        
        assert_eq!(actor_query["total"], 1);
        assert_eq!(actor_query["records"][0]["actor"], *actor);
        assert_eq!(actor_query["records"][0]["action"], *action);
        assert_eq!(actor_query["records"][0]["target"], *target);
    }
}

#[tokio::test]
async fn test_sit_performance_benchmarks() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    // Benchmark 1: Single request latency
    let start = std::time::Instant::now();
    
    let request = json!({
        "actor": "PerformanceTest",
        "action": "latency_test",
        "target": "benchmark",
        "record_type": "Temporal"
    });
    
    let response = server.add_memory(request).await
        .expect("Performance test failed");
    
    let latency = start.elapsed();
    
    assert_eq!(response["success"], true);
    assert!(latency < Duration::from_millis(1000), 
           "Single request should complete within 1 second, took {:?}", latency);
    
    // Benchmark 2: Query performance
    let query_start = std::time::Instant::now();
    
    let query_response = server.query_memory("actor=PerformanceTest").await
        .expect("Query performance test failed");
    
    let query_latency = query_start.elapsed();
    
    assert_eq!(query_response["total"], 1);
    assert!(query_latency < Duration::from_millis(500),
           "Query should complete within 500ms, took {:?}", query_latency);
    
    // Benchmark 3: Bulk operations
    let bulk_start = std::time::Instant::now();
    
    for i in 0..100 {
        let bulk_request = json!({
            "actor": format!("BulkTest_{}", i),
            "action": "bulk_operation",
            "target": "performance_test",
            "record_type": "Temporal"
        });
        
        server.add_memory(bulk_request).await
            .expect("Bulk operation failed");
    }
    
    let bulk_duration = bulk_start.elapsed();
    let avg_per_request = bulk_duration / 100;
    
    assert!(avg_per_request < Duration::from_millis(100),
           "Average request time should be under 100ms, was {:?}", avg_per_request);
    
    // Verify all bulk records were added
    let bulk_query = server.query_memory("action=bulk_operation").await
        .expect("Bulk query failed");
    
    assert_eq!(bulk_query["total"], 100);
}

#[tokio::test]
async fn test_sit_api_versioning_and_compatibility() {
    let server = TestServer::start().await
        .expect("Failed to start test server");
    
    let client = reqwest::Client::new();
    
    // Test API endpoints exist and respond correctly
    let endpoints = vec![
        ("/health", "GET"),
        ("/memory/add", "POST"),
        ("/memory/query", "GET"),
    ];
    
    for (endpoint, method) in endpoints {
        let url = format!("{}{}", server.base_url, endpoint);
        
        let response = match method {
            "GET" => client.get(&url).send().await,
            "POST" => {
                let test_data = json!({
                    "actor": "CompatibilityTest",
                    "action": "api_test",
                    "target": "endpoint_validation",
                    "record_type": "Temporal"
                });
                client.post(&url).json(&test_data).send().await
            },
            _ => panic!("Unsupported method"),
        };
        
        let response = response.expect(&format!("Failed to call {} {}", method, endpoint));
        
        // Should not return 404
        assert_ne!(response.status(), reqwest::StatusCode::NOT_FOUND,
                  "Endpoint {} {} should exist", method, endpoint);
        
        // Should return valid response format
        if response.status().is_success() {
            if endpoint != "/health" {
                let _: serde_json::Value = response.json().await
                    .expect(&format!("Response from {} {} should be valid JSON", method, endpoint));
            }
        }
    }
}
