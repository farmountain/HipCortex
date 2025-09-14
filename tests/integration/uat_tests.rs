use serde_json::json;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use tempfile::tempdir;
use tokio;
use uuid::Uuid;

use hipcortex::decay::DecayType;
use hipcortex::memory_store::MemoryStore;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};

pub struct UATTestRunner {
    server_process: Option<Child>,
    base_url: String,
    vscode_extension_path: String,
}

impl UATTestRunner {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let memory_file = temp_dir.path().join("uat_memory.jsonl");
        
        // Start HipCortex server
        let server_process = Command::new("cargo")
            .args(&[
                "run",
                "--bin", "webserver",
                "--features", "web-server,petgraph_backend"
            ])
            .env("MEMORY_FILE", memory_file.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let base_url = "http://127.0.0.1:3030".to_string();
        let vscode_extension_path = "d:\\All_Projects\\HipCortex\\HipCortex\\vscode-extension".to_string();
        
        // Wait for server to be ready with longer timeout
        let client = reqwest::Client::new();
        for i in 0..60 {  // Increased from 30 to 60 attempts
            thread::sleep(Duration::from_millis(1000));  // Increased from 500ms to 1000ms
            if let Ok(response) = client.get(&format!("{}/health", base_url)).send().await {
                if response.status().is_success() {
                    println!("✅ Server ready after {} seconds", i + 1);
                    return Ok(UATTestRunner {
                        server_process: Some(server_process),
                        base_url,
                        vscode_extension_path,
                    });
                }
            }
            if i % 10 == 0 {
                println!("⏳ Waiting for server... attempt {}/60", i + 1);
            }
        }

        Err("Server failed to start within timeout".into())
    }

    async fn api_call(&self, method: &str, endpoint: &str, data: Option<serde_json::Value>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = match method {
            "GET" => client.get(&url).send().await?,
            "POST" => {
                let req = client.post(&url);
                if let Some(json_data) = data {
                    req.json(&json_data).send().await?
                } else {
                    req.send().await?
                }
            },
            _ => return Err("Unsupported HTTP method".into()),
        };

        Ok(response.json().await?)
    }

    async fn simulate_user_workflow(&self, scenario: &str) -> Result<UATResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut steps_completed = 0;
        let mut errors = Vec::new();

        match scenario {
            "developer_workflow" => {
                // Step 1: Developer opens VS Code with HipCortex extension
                steps_completed += 1;
                
                // Step 2: Developer writes code and wants to capture learning
                let memory_request = json!({
                    "actor": "developer_alice",
                    "action": "learned_rust_pattern",
                    "target": "async_error_handling",
                    "record_type": "Procedural",
                    "metadata": {
                        "file": "src/memory_store.rs",
                        "line": 42,
                        "pattern": "Result<T, E> with ? operator",
                        "confidence": 0.95
                    }
                });

                match self.api_call("POST", "/memory/add", Some(memory_request)).await {
                    Ok(response) => {
                        if response["success"] == true {
                            steps_completed += 1;
                        } else {
                            errors.push("Failed to add procedural memory".to_string());
                        }
                    }
                    Err(e) => errors.push(format!("API call failed: {}", e)),
                }

                // Step 3: Developer queries for related patterns
                match self.api_call("GET", "/memory/query?target=async", None).await {
                    Ok(response) => {
                        if response["total"].as_u64().unwrap_or(0) > 0 {
                            steps_completed += 1;
                        } else {
                            errors.push("No related patterns found".to_string());
                        }
                    }
                    Err(e) => errors.push(format!("Query failed: {}", e)),
                }

                // Step 4: Developer uses @hipcortex chat to get suggestions
                // Simulated through API calls since we can't directly test VS Code
                let context_query = json!({
                    "actor": "developer_alice",
                    "action": "request_context",
                    "target": "error_handling_patterns",
                    "record_type": "Temporal"
                });

                match self.api_call("POST", "/memory/add", Some(context_query)).await {
                    Ok(_) => steps_completed += 1,
                    Err(e) => errors.push(format!("Context request failed: {}", e)),
                }
            }

            "student_workflow" => {
                // Step 1: Student starts learning session
                let learning_start = json!({
                    "actor": "student_bob",
                    "action": "start_learning_session",
                    "target": "rust_ownership",
                    "record_type": "Temporal",
                    "metadata": {
                        "session_type": "focused_study",
                        "duration_planned": 120
                    }
                });

                match self.api_call("POST", "/memory/add", Some(learning_start)).await {
                    Ok(_) => steps_completed += 1,
                    Err(e) => errors.push(format!("Learning session start failed: {}", e)),
                }

                // Step 2: Student encounters and solves problems
                for i in 1..=5 {
                    let problem_solving = json!({
                        "actor": "student_bob",
                        "action": "solved_problem",
                        "target": format!("ownership_exercise_{}", i),
                        "record_type": "Procedural",
                        "metadata": {
                            "difficulty": if i <= 2 { "beginner" } else if i <= 4 { "intermediate" } else { "advanced" },
                            "time_taken": 15 * i,
                            "hints_used": std::cmp::max(0, 3 - i)
                        }
                    });

                    match self.api_call("POST", "/memory/add", Some(problem_solving)).await {
                        Ok(_) => steps_completed += 1,
                        Err(e) => errors.push(format!("Problem {} logging failed: {}", i, e)),
                    }
                }

                // Step 3: Student reviews progress
                match self.api_call("GET", "/memory/query?actor=student_bob&action=solved_problem", None).await {
                    Ok(response) => {
                        if response["total"].as_u64().unwrap_or(0) >= 5 {
                            steps_completed += 1;
                        } else {
                            errors.push("Progress review incomplete".to_string());
                        }
                    }
                    Err(e) => errors.push(format!("Progress review failed: {}", e)),
                }
            }

            "researcher_workflow" => {
                // Step 1: Researcher starts investigation
                let research_start = json!({
                    "actor": "researcher_carol",
                    "action": "begin_investigation", 
                    "target": "memory_consolidation_patterns",
                    "record_type": "Reflexive",
                    "metadata": {
                        "hypothesis": "Spaced repetition improves long-term retention",
                        "methodology": "longitudinal_study"
                    }
                });

                match self.api_call("POST", "/memory/add", Some(research_start)).await {
                    Ok(_) => steps_completed += 1,
                    Err(e) => errors.push(format!("Research start failed: {}", e)),
                }

                // Step 2: Collect data points
                for day in 1..=7 {
                    let data_point = json!({
                        "actor": "researcher_carol",
                        "action": "collect_data_point",
                        "target": format!("day_{}_observations", day),
                        "record_type": "Temporal",
                        "metadata": {
                            "retention_score": 0.7 + (day as f64 * 0.03),
                            "recall_time": 2.5 - (day as f64 * 0.1),
                            "confidence": 0.8 + (day as f64 * 0.02)
                        }
                    });

                    match self.api_call("POST", "/memory/add", Some(data_point)).await {
                        Ok(_) => steps_completed += 1,
                        Err(e) => errors.push(format!("Day {} data collection failed: {}", day, e)),
                    }
                }

                // Step 3: Analyze patterns
                match self.api_call("GET", "/memory/query?actor=researcher_carol&action=collect_data_point", None).await {
                    Ok(response) => {
                        if response["total"].as_u64().unwrap_or(0) >= 7 {
                            steps_completed += 1;
                        } else {
                            errors.push("Insufficient data for analysis".to_string());
                        }
                    }
                    Err(e) => errors.push(format!("Data analysis failed: {}", e)),
                }
            }

            _ => return Err(format!("Unknown scenario: {}", scenario).into()),
        }

        Ok(UATResult {
            scenario: scenario.to_string(),
            duration: start_time.elapsed(),
            steps_completed,
            success: errors.is_empty(),
            errors,
        })
    }
}

impl Drop for UATTestRunner {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

#[derive(Debug)]
pub struct UATResult {
    pub scenario: String,
    pub duration: Duration,
    pub steps_completed: usize,
    pub errors: Vec<String>,
    pub success: bool,
}

#[tokio::test]
async fn test_uat_developer_workflow() {
    let runner = UATTestRunner::new().await
        .expect("Failed to setup UAT runner");

    let result = runner.simulate_user_workflow("developer_workflow").await
        .expect("Developer workflow test failed");

    println!("Developer Workflow UAT Results:");
    println!("  Duration: {:?}", result.duration);
    println!("  Steps Completed: {}", result.steps_completed);
    println!("  Success: {}", result.success);
    
    if !result.errors.is_empty() {
        println!("  Errors:");
        for error in &result.errors {
            println!("    - {}", error);
        }
    }

    assert!(result.success, "Developer workflow should complete successfully");
    assert!(result.steps_completed >= 4, "Should complete all major workflow steps");
    assert!(result.duration < Duration::from_secs(30), "Workflow should complete within 30 seconds");
}

#[tokio::test]
async fn test_uat_student_workflow() {
    let runner = UATTestRunner::new().await
        .expect("Failed to setup UAT runner");

    let result = runner.simulate_user_workflow("student_workflow").await
        .expect("Student workflow test failed");

    println!("Student Workflow UAT Results:");
    println!("  Duration: {:?}", result.duration);
    println!("  Steps Completed: {}", result.steps_completed);
    println!("  Success: {}", result.success);
    
    if !result.errors.is_empty() {
        println!("  Errors:");
        for error in &result.errors {
            println!("    - {}", error);
        }
    }

    assert!(result.success, "Student workflow should complete successfully");
    assert!(result.steps_completed >= 7, "Should complete learning session and problem solving");
    assert!(result.duration < Duration::from_secs(45), "Workflow should complete within 45 seconds");
}

#[tokio::test]
async fn test_uat_researcher_workflow() {
    let runner = UATTestRunner::new().await
        .expect("Failed to setup UAT runner");

    let result = runner.simulate_user_workflow("researcher_workflow").await
        .expect("Researcher workflow test failed");

    println!("Researcher Workflow UAT Results:");
    println!("  Duration: {:?}", result.duration);
    println!("  Steps Completed: {}", result.steps_completed);
    println!("  Success: {}", result.success);
    
    if !result.errors.is_empty() {
        println!("  Errors:");
        for error in &result.errors {
            println!("    - {}", error);
        }
    }

    assert!(result.success, "Researcher workflow should complete successfully");
    assert!(result.steps_completed >= 9, "Should complete research setup, data collection, and analysis");
    assert!(result.duration < Duration::from_secs(60), "Workflow should complete within 60 seconds");
}

#[test]
fn athena_reflexion_placeholder() {
    use hipcortex::aureus_bridge::AureusBridge;
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_uat_mem.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
}

#[test]
fn athena_chain_of_thought_reasoning() {
    use hipcortex::aureus_bridge::{AureusBridge, AureusConfig};
    use hipcortex::llm_clients::mock::MockClient;
    let path = "test_uat_cot.jsonl";
    let _ = std::fs::remove_file(path);
    let mut aureus = AureusBridge::with_client(Box::new(MockClient));
    aureus.configure(AureusConfig {
        enable_cot: true,
        prune_threshold: 0.2,
    });
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
    assert!(store.all()[0].target.contains("Think step by step."));
    std::fs::remove_file(path).ok();
}

#[test]
fn user_store_reasoning_trace() {
    use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};

    let mut indexer = TemporalIndexer::new(4, 3600);

    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("UAT reasoning".to_string()),
        embedding: None,
        image_data: None,
        tags: vec!["uat".into()],
    };
    let out = PerceptionAdapter::adapt(input.clone()).unwrap();
    assert_eq!(out.len(), 4);

    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: input.text.unwrap(),
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential {
            half_life: Duration::from_secs(1),
        },
    };
    indexer.insert(trace);

    let last_trace = indexer.get_recent(1)[0];
    assert_eq!(last_trace.data, "UAT reasoning");
}

#[test]
fn user_query_city_by_label() {
    let mut store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "city".to_string());
    store.add_node("London", props.clone());
    let results = store.find_by_label("London");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].properties.get("type").unwrap(), "city");
}
