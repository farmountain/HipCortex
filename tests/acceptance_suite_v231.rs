//! Acceptance suite v2.3.1 — Harness Alignment
//!
//! AC-H1: harness.md does NOT instruct host to POST /memory/add for env observations.
//! AC-H2: harness.md documents /intent/open and /intent/receipt as the only env API.
//! AC-H3: web_server.rs has POST /intent/open route wired to CognitiveDelta::OpenIntent.
//! AC-H4: web_server.rs has POST /intent/receipt route wired to CognitiveDelta::AcceptReceipt.
//! AC-H5: sdk/mcp/server.py has open_intent and accept_receipt tools in dispatch table.

use std::process;

macro_rules! ac {
    ($label:expr, $body:block) => {{
        let result = std::panic::catch_unwind(|| $body);
        match result {
            Ok(_) => println!("[PASS] {}", $label),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("[FAIL] {} — {}", $label, msg);
                process::exit(1);
            }
        }
    }};
}

fn read_file(rel: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(rel))
        .unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

fn main() {
    // ── AC-H1 ─────────────────────────────────────────────────────────────────
    ac!("AC-H1 harness.md does NOT instruct POST /memory/add for env observations", {
        let harness = read_file("docs/harness.md");
        // The harness should not have a loop step telling the host to use add_memory for env obs.
        // The old pattern was "STORE: POST /memory/add (Temporal)" as the only env storage step.
        // v2.3.0 replaces that with the accept_receipt seam.
        assert!(
            !harness.contains("STORE: POST /memory/add (Temporal)"),
            "harness.md still tells host to use POST /memory/add as the env obs storage step"
        );
        assert!(
            !harness.contains("STORE:   POST /memory/add (Temporal)"),
            "harness.md still tells host to use POST /memory/add as the env obs storage step"
        );
    });

    // ── AC-H2 ─────────────────────────────────────────────────────────────────
    ac!("AC-H2 harness.md documents Intent/Receipt as the env API seam", {
        let harness = read_file("docs/harness.md");
        assert!(
            harness.contains("/intent/open"),
            "harness.md must document /intent/open endpoint"
        );
        assert!(
            harness.contains("/intent/receipt"),
            "harness.md must document /intent/receipt endpoint"
        );
        assert!(
            harness.contains("GroundingGate"),
            "harness.md must document GroundingGate"
        );
        assert!(
            harness.contains("accept_receipt"),
            "harness.md must document accept_receipt as the env obs path"
        );
        assert!(
            harness.contains("accept_receipt ONLY") || harness.contains("only via accept_receipt")
                || harness.contains("never add_memory") || harness.contains("Never"),
            "harness.md must warn that env observations must NOT use add_memory"
        );
    });

    // ── AC-H3 ─────────────────────────────────────────────────────────────────
    ac!("AC-H3 web_server.rs has POST /intent/open wired to CognitiveDelta::OpenIntent", {
        let ws = read_file("src/web_server.rs");
        assert!(
            ws.contains("\"/intent/open\""),
            "web_server.rs must register /intent/open route"
        );
        assert!(
            ws.contains("CognitiveDelta::OpenIntent"),
            "web_server.rs must wire /intent/open to CognitiveDelta::OpenIntent"
        );
    });

    // ── AC-H4 ─────────────────────────────────────────────────────────────────
    ac!("AC-H4 web_server.rs has POST /intent/receipt wired to CognitiveDelta::AcceptReceipt", {
        let ws = read_file("src/web_server.rs");
        assert!(
            ws.contains("\"/intent/receipt\""),
            "web_server.rs must register /intent/receipt route"
        );
        assert!(
            ws.contains("CognitiveDelta::AcceptReceipt"),
            "web_server.rs must wire /intent/receipt to CognitiveDelta::AcceptReceipt"
        );
    });

    // ── AC-H5 ─────────────────────────────────────────────────────────────────
    ac!("AC-H5 sdk/mcp/server.py has open_intent and accept_receipt in dispatch", {
        let mcp = read_file("sdk/mcp/server.py");
        assert!(
            mcp.contains("\"open_intent\"") || mcp.contains("'open_intent'"),
            "server.py must register open_intent tool"
        );
        assert!(
            mcp.contains("\"accept_receipt\"") || mcp.contains("'accept_receipt'"),
            "server.py must register accept_receipt tool"
        );
        assert!(
            mcp.contains("handle_open_intent"),
            "server.py must have handle_open_intent handler"
        );
        assert!(
            mcp.contains("handle_accept_receipt"),
            "server.py must have handle_accept_receipt handler"
        );
        assert!(
            mcp.contains("/intent/open"),
            "handle_open_intent must call /intent/open REST endpoint"
        );
        assert!(
            mcp.contains("/intent/receipt"),
            "handle_accept_receipt must call /intent/receipt REST endpoint"
        );
    });

    println!("\n=== Acceptance v2.3.1 (Harness Alignment): 5/5 passed ===");
}
