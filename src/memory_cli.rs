use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::llm_clients::LLMClient;
use crate::memory_processor::MemoryProcessor;
use crate::memory_query::MemoryQuery;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::snapshot_manager::SnapshotManager;

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
            let archive = PathBuf::from(format!("{}.tar.gz", tag));
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
    }
    Ok(())
}
