use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::llm_clients::{LLMClient, LanguageModelClient};
use crate::memory_processor::MemoryProcessor;
use crate::memory_query::MemoryQuery;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::snapshot_manager::SnapshotManager;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "hipcortex", version, about = "Minimal Memory CLI")]
struct Cli {
    /// Path to memory store file
    #[arg(long, default_value = "memory.jsonl")]
    store: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a memory record
    Add {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        target: String,
    },
    /// Query records
    Query {
        #[arg(long)]
        r#type: Option<MemoryType>,
        #[arg(long)]
        actor: Option<String>,
        #[arg(short = 'q', long = "query")]
        query: Option<String>,
        #[arg(long)]
        since: Option<DateTime<Utc>>,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long, default_value_t = 10)]
        page_size: usize,
    },
    /// Save snapshot
    Snapshot { tag: String },
    /// Restore snapshot
    Restore { tag: String },
    /// Send a prompt to OpenAI and store reflexion
    Prompt { prompt: String },
    /// Export symbolic world model as JSON
    Graph {
        #[arg(long, default_value = "graph.db")]
        db: String,
    },
    /// Put an embedding into semantic cache file
    CachePut {
        #[arg(long, default_value = "cache.json")]
        cache: String,
        key: String,
        #[arg(long)]
        embedding: String,
        #[arg(long, default_value_t = 16)]
        capacity: usize,
    },
    /// Get nearest embedding from cache file
    CacheGet {
        #[arg(long, default_value = "cache.json")]
        cache: String,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 16)]
        capacity: usize,
    },
    /// Generate text using a local or mocked LLM
    LlmGenerate { prompt: String },
    /// Predict next state using the world model connector
    WorldModelPredict { input: String },
    /// Run a demonstration workflow
    RunWorkflow,
    /// Show recent safety audit snapshots
    SafetyAudit,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut store = MemoryStore::new(&cli.store)?;
    match cli.command {
        Commands::Add {
            actor,
            action,
            target,
        } => {
            let record = MemoryRecord::new(
                MemoryType::Temporal,
                actor,
                action,
                target,
                serde_json::json!({}),
            );
            store.add(record)?;
            // Keep on-disk store deduplicated when adding records
            let mut all = store.all().to_vec();
            MemoryProcessor::deduplicate(&mut all);
        }
        Commands::Query {
            r#type,
            actor,
            query,
            since,
            page,
            page_size,
        } => {
            let mut data: Vec<MemoryRecord> = store.all().to_vec();
            if let Some(t) = r#type {
                data = MemoryQuery::by_type(&data, t)
                    .into_iter()
                    .cloned()
                    .collect();
            }
            if let Some(a) = actor {
                data = MemoryQuery::by_actor(&data, &a)
                    .into_iter()
                    .cloned()
                    .collect();
            }
            if let Some(q) = query {
                data = MemoryQuery::search(&data, &q)
                    .into_iter()
                    .cloned()
                    .collect();
            }
            if let Some(ts) = since {
                data = MemoryQuery::since(&data, ts).into_iter().cloned().collect();
            }
            let page = page.unwrap_or(1).saturating_sub(1);
            let start = page * page_size;
            let end = (start + page_size).min(data.len());
            for r in data[start..end].iter() {
                println!("{:?}", r);
            }
        }
        Commands::Snapshot { tag } => {
            SnapshotManager::save(&cli.store, &tag)?;
        }
        Commands::Restore { tag } => {
            let archive = PathBuf::from(&cli.store).with_extension(format!("{}.tar.gz", tag));
            SnapshotManager::load(&archive, Path::new("."))?;
        }
        Commands::Prompt { prompt } => {
            use crate::llm_clients::openai::OpenAIClient;
            let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            let client = OpenAIClient::new(api_key, "gpt-3.5-turbo");
            let response = client.generate_response(&prompt);
            let record = MemoryRecord::new(
                MemoryType::Reflexion,
                "cli".into(),
                "prompt".into(),
                response.clone(),
                serde_json::json!({ "prompt": prompt }),
            );
            store.add(record)?;
            println!("{}", response);
        }
        Commands::Graph { db } => {
            use crate::symbolic_store::{SledGraph, SymbolicStore};
            let backend = SledGraph::open(db)?;
            let store = SymbolicStore::from_backend(backend);
            let graph = store.export_graph();
            println!("{}", serde_json::to_string(&graph)?);
        }
        Commands::CachePut {
            cache,
            key,
            embedding,
            capacity,
        } => {
            use crate::semantic_cache::SemanticCache;
            let mut sc = SemanticCache::load(&cache, capacity);
            let vec: Vec<f32> = embedding
                .split(',')
                .filter_map(|v| v.parse::<f32>().ok())
                .collect();
            sc.put_embedding(key, vec);
            sc.save(&cache)?;
        }
        Commands::CacheGet {
            cache,
            query,
            capacity,
        } => {
            use crate::semantic_cache::SemanticCache;
            let mut sc = SemanticCache::load(&cache, capacity);
            let vec: Vec<f32> = query
                .split(',')
                .filter_map(|v| v.parse::<f32>().ok())
                .collect();
            if let Some((k, v)) = sc.get_nearest(&vec) {
                println!("{} {:?}", k, v);
            }
            sc.save(&cache)?;
        }
        Commands::LlmGenerate { prompt } => {
            use crate::llm_clients::local_llm_client::LocalLLMClient;
            let cmd = std::env::var("LOCAL_LLM_CMD").unwrap_or_else(|_| "echo".into());
            let client = LocalLLMClient::new(cmd);
            let resp = client.generate(&prompt);
            println!("{}", resp);
        }
        Commands::WorldModelPredict { input } => {
            use crate::world_model_client::{MockWorldModelClient, WorldModelClient};
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&input) {
                let state = v["state"].as_str().unwrap_or("");
                let action = v["action"].as_str().unwrap_or("");
                let client = MockWorldModelClient;
                let out = client.predict_next_state(state, action);
                println!("{}", out);
            } else {
                println!("invalid input");
            }
        }
        Commands::RunWorkflow => {
            use crate::backends::temporal_backend::TemporalFSMBackend;
            use crate::procedural_cache::{
                FSMState, FSMTransition, ProceduralCache, ProceduralTrace,
            };
            use std::collections::HashMap;
            let backend = TemporalFSMBackend::new();
            let mut cache = ProceduralCache::from_backend(backend);
            let trace = ProceduralTrace {
                id: Uuid::new_v4(),
                current_state: FSMState::Start,
                memory: HashMap::new(),
            };
            cache.add_trace(trace.clone());
            cache.add_transition(FSMTransition {
                from: FSMState::Start,
                to: FSMState::Observe,
                condition: None,
            });
            cache.add_transition(FSMTransition {
                from: FSMState::Observe,
                to: FSMState::Act,
                condition: None,
            });
            cache.add_transition(FSMTransition {
                from: FSMState::Act,
                to: FSMState::End,
                condition: None,
            });
            let mut state = trace.current_state;
            println!("state: {:?}", state);
            while state != FSMState::End {
                state = cache.advance(trace.id, None).unwrap();
                println!("state: {:?}", state);
            }
        }
        Commands::SafetyAudit => {
            let snaps = {
                let guard = crate::safety_guardrail::SAFETY_GUARDRAIL.lock().unwrap();
                guard.recent_snapshots(5)
            };
            println!("{}", serde_json::to_string_pretty(&snaps)?);
        }
    }
    Ok(())
}
