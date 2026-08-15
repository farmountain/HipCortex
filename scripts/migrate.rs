//! Minimal HipCortex store migration helper.
//! Usage: cargo run --bin migrate -- <from_path.jsonl> <to_path.jsonl>
//! Copies JSONL lines, validating each is a JSON object with an "id" field when present.

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: migrate <source.jsonl> <dest.jsonl>");
        std::process::exit(2);
    }
    let src = Path::new(&args[1]);
    let dst = Path::new(&args[2]);
    if !src.exists() {
        eprintln!("error: source not found: {}", src.display());
        std::process::exit(1);
    }
    let reader = BufReader::new(File::open(src).expect("open source"));
    let mut out = File::create(dst).expect("create dest");
    let mut ok = 0usize;
    let mut skip = 0usize;
    for line in reader.lines() {
        let line = line.expect("read");
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(t) {
            Ok(v) if v.is_object() => {
                writeln!(out, "{}", t).expect("write");
                ok += 1;
            }
            _ => {
                eprintln!("skip invalid line: {}", &t[..t.len().min(80)]);
                skip += 1;
            }
        }
    }
    println!(
        "migrated {} records → {} (skipped {})",
        ok,
        dst.display(),
        skip
    );
}
