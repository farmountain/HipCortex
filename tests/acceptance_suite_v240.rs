//! Acceptance suite v2.4.0 — Published Runner
//!
//! AC-R1: cli.py registers a `runner` subcommand.
//! AC-R2: runner.py has `poll_and_execute` method on IntentRunner.
//! AC-R3: runner.py has `execute_probe` dispatching filesystem/http/shell/default.
//! AC-R4: runner.py skips expired intents (deadline_ms check before dispatch).
//! AC-R5: RUNNER_SKILL.md exists and documents the IDE-connected runner protocol.

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
    // ── AC-R1 ─────────────────────────────────────────────────────────────────
    ac!("AC-R1 cli.py registers runner subcommand", {
        let cli = read_file("sdk/python/hipcortex/cli.py");
        assert!(
            cli.contains("\"runner\"") || cli.contains("'runner'"),
            "cli.py must register a runner subcommand"
        );
        assert!(
            cli.contains("cmd_runner"),
            "cli.py must have a cmd_runner handler"
        );
        assert!(
            cli.contains("hipcortex-runner"),
            "cli.py runner subcommand must set default actor to hipcortex-runner"
        );
    });

    // ── AC-R2 ─────────────────────────────────────────────────────────────────
    ac!("AC-R2 runner.py has IntentRunner with poll_and_execute", {
        let runner = read_file("sdk/python/hipcortex/runner.py");
        assert!(
            runner.contains("class IntentRunner"),
            "runner.py must define IntentRunner class"
        );
        assert!(
            runner.contains("def poll_and_execute"),
            "runner.py must define poll_and_execute method"
        );
        assert!(
            runner.contains("def run_forever"),
            "runner.py must define run_forever method"
        );
        assert!(
            runner.contains("/intent/open"),
            "runner.py must poll /intent/open endpoint"
        );
        assert!(
            runner.contains("/intent/receipt"),
            "runner.py must post to /intent/receipt endpoint"
        );
    });

    // ── AC-R3 ─────────────────────────────────────────────────────────────────
    ac!("AC-R3 runner.py execute_probe dispatches filesystem/http/shell/default", {
        let runner = read_file("sdk/python/hipcortex/runner.py");
        assert!(
            runner.contains("def execute_probe"),
            "runner.py must define execute_probe method"
        );
        assert!(
            runner.contains("filesystem"),
            "execute_probe must handle filesystem sensor_path"
        );
        assert!(
            runner.contains("_probe_filesystem"),
            "runner.py must have _probe_filesystem function"
        );
        assert!(
            runner.contains("http"),
            "execute_probe must handle http sensor_path"
        );
        assert!(
            runner.contains("_probe_http"),
            "runner.py must have _probe_http function"
        );
        assert!(
            runner.contains("shell"),
            "execute_probe must handle shell sensor_path"
        );
        assert!(
            runner.contains("reachable"),
            "execute_probe default branch must return reachable: True"
        );
    });

    // ── AC-R4 ─────────────────────────────────────────────────────────────────
    ac!("AC-R4 runner.py skips expired intents", {
        let runner = read_file("sdk/python/hipcortex/runner.py");
        assert!(
            runner.contains("deadline_ms"),
            "runner.py must check deadline_ms for expiry"
        );
        assert!(
            runner.contains("expired") || runner.contains("skip expired"),
            "runner.py must log or comment skipping expired intents"
        );
        assert!(
            runner.contains("_now_ms") || runner.contains("now_ms"),
            "runner.py must compute current time in ms for expiry check"
        );
    });

    // ── AC-R5 ─────────────────────────────────────────────────────────────────
    ac!("AC-R5 RUNNER_SKILL.md exists and documents IDE runner protocol", {
        let skill = read_file("sdk/python/hipcortex/install/RUNNER_SKILL.md");
        assert!(
            skill.contains("/intent/open"),
            "RUNNER_SKILL.md must document /intent/open polling"
        );
        assert!(
            skill.contains("/intent/receipt"),
            "RUNNER_SKILL.md must document /intent/receipt posting"
        );
        assert!(
            skill.contains("accept_receipt") || skill.contains("AcceptReceipt"),
            "RUNNER_SKILL.md must reference accept_receipt as the env obs path"
        );
        assert!(
            skill.contains("add_memory") && (skill.contains("NOT") || skill.contains("not") || skill.contains("never") || skill.contains("Never")),
            "RUNNER_SKILL.md must warn against using add_memory for env observations"
        );
        assert!(
            skill.contains("deadline_ms") || skill.contains("expired") || skill.contains("Skip"),
            "RUNNER_SKILL.md must document skipping expired intents"
        );
    });

    println!("\n=== Acceptance v2.4.0 (Published Runner): 5/5 passed ===");
}
