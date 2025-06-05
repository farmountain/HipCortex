use anyhow::Result;
use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

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
        #[arg(long)]
        since: Option<DateTime<Utc>>,
    },
    /// Save snapshot
    Snapshot {
        tag: String,
    },
    /// Restore snapshot
    Restore {
        tag: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut store = MemoryStore::new(&cli.store)?;
    match cli.command {
        Commands::Add { actor, action, target } => {
            let record = MemoryRecord::new(
                MemoryType::Temporal,
                actor,
                action,
                target,
                serde_json::json!({}),
            );
            store.add(record)?;
        }
        Commands::Query { r#type, actor, since } => {
            let mut results: Vec<&MemoryRecord> = store.all().iter().collect();
            if let Some(t) = r#type {
                results = results.into_iter().filter(|r| r.record_type == t).collect();
            }
            if let Some(a) = actor {
                results = results.into_iter().filter(|r| r.actor == a).collect();
            }
            if let Some(ts) = since {
                results = results.into_iter().filter(|r| r.timestamp >= ts).collect();
            }
            for r in results {
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
    }
    Ok(())
}
