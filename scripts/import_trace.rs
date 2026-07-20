//! Import a simple trace JSON array into HipCortex memory JSONL.
//! Usage: cargo run --bin import_trace -- <trace.json> <out.jsonl>
//! Trace format: [{"actor":"...","action":"...","target":"..."}, ...]

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: import_trace <trace.json> <out.jsonl>");
        std::process::exit(2);
    }
    let src = Path::new(&args[1]);
    let dst = Path::new(&args[2]);
    let raw = fs::read_to_string(src).expect("read trace");
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&raw).expect("trace must be a JSON array");
    let mut out = fs::File::create(dst).expect("create out");
    let mut n = 0usize;
    for item in arr {
        let actor = item
            .get("actor")
            .and_then(|v| v.as_str())
            .unwrap_or("import");
        let action = item
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("observed");
        let target = item
            .get("target")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("text").and_then(|v| v.as_str()))
            .unwrap_or("");
        if target.is_empty() {
            continue;
        }
        let rec = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "record_type": "Temporal",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "actor": actor,
            "action": action,
            "target": target,
            "metadata": item.get("metadata").cloned().unwrap_or(serde_json::json!({})),
            "status": "active"
        });
        writeln!(out, "{}", rec).expect("write");
        n += 1;
    }
    println!("imported {} records → {}", n, dst.display());
}
