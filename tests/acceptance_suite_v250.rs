//! Acceptance suite v2.5.0 — IG Probe Ranking + add_memory Adapter
//!
//! Gap 3: Information-Gain Probe Ranking
//!   AC-P1: action_intent.rs defines ig_score() with epistemic × deficit × probe_penalty
//!   AC-P2: entity at n=0 outranks entity at n=2 (both ungrounded)
//!   AC-P3: entity at n≥4 (MAPPED) gets score=0.0 — never probe-targeted
//!   AC-P4: high probe_count deprioritises entity vs same-n with probe_count=0
//!   AC-P5: grounding_gate.rs ig_probe_target returns None when all entities grounded
//!
//! Gap 4: add_memory → AcceptReceipt Adapter
//!   AC-A1: AddMemoryRequest has intent_id field (web_server.rs)
//!   AC-A2: handle_add_memory returns 400 redirect when intent_id + Temporal
//!   AC-A3: MCP add_memory routes to accept_receipt when intent_id present
//!   AC-A4: Python SDK add_memory routes to /intent/receipt when intent_id set
//!   AC-A5: harness.md documents adapter routing (SDK/MCP + REST 400)

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

// ── In-process unit tests for Gap 3 (no server needed) ───────────────────────

fn make_contact(n_obs: u32, probe_count: u32) -> hipcortex::action_intent::EntityContactRecord {
    hipcortex::action_intent::EntityContactRecord {
        n_observations: n_obs,
        probe_count,
        ..Default::default()
    }
}

fn main() {
    // ── AC-P1 ─────────────────────────────────────────────────────────────────
    ac!("AC-P1 action_intent.rs defines ig_score with all 3 weight components", {
        let src = read_file("src/action_intent.rs");
        assert!(src.contains("fn ig_score"), "missing ig_score method");
        assert!(src.contains("deficit"), "missing deficit weight");
        assert!(src.contains("probe_penalty"), "missing probe_penalty weight");
        assert!(src.contains("epistemic()"), "must call epistemic() inside ig_score");
        assert!(src.contains("MAPPED_OBS_THRESHOLD"), "deficit must reference MAPPED_OBS_THRESHOLD");
    });

    // ── AC-P2 ─────────────────────────────────────────────────────────────────
    ac!("AC-P2 n=0 entity ranks above n=2 entity (both ungrounded)", {
        let c0 = make_contact(0, 0);
        let c2 = make_contact(2, 0);
        assert!(
            c0.ig_score() > c2.ig_score(),
            "n=0 ig_score={} must be > n=2 ig_score={}",
            c0.ig_score(), c2.ig_score()
        );
    });

    // ── AC-P3 ─────────────────────────────────────────────────────────────────
    ac!("AC-P3 n>=4 (MAPPED) entity has ig_score=0.0", {
        use hipcortex::action_intent::MAPPED_OBS_THRESHOLD;
        let grounded = make_contact(MAPPED_OBS_THRESHOLD, 0);
        assert_eq!(
            grounded.ig_score(), 0.0,
            "grounded entity must have ig_score=0.0 to prevent reprobe"
        );
        let over = make_contact(MAPPED_OBS_THRESHOLD + 2, 0);
        assert_eq!(over.ig_score(), 0.0, "over-threshold entity must also be 0.0");
    });

    // ── AC-P4 ─────────────────────────────────────────────────────────────────
    ac!("AC-P4 high probe_count deprioritises entity vs same-n peer", {
        let fresh  = make_contact(1, 0);
        let stale  = make_contact(1, 5);
        assert!(
            fresh.ig_score() > stale.ig_score(),
            "n=1,probe=0 score={} must exceed n=1,probe=5 score={}",
            fresh.ig_score(), stale.ig_score()
        );
    });

    // ── AC-P5 ─────────────────────────────────────────────────────────────────
    ac!("AC-P5 ig_probe_target returns None when all entities grounded", {
        use hipcortex::action_intent::MAPPED_OBS_THRESHOLD;
        use hipcortex::grounding_gate::GroundingGate;
        let entities = ["a", "b", "c"];
        let entity_refs: Vec<&str> = entities.iter().map(|s| *s).collect();
        let contact_fn = |_name: &str| {
            Some(make_contact(MAPPED_OBS_THRESHOLD, 0))
        };
        let result = GroundingGate::ig_probe_target(&entity_refs, contact_fn);
        assert!(
            result.is_none(),
            "ig_probe_target must return None when all entities are grounded, got {:?}", result
        );
    });

    // ── AC-A1 ─────────────────────────────────────────────────────────────────
    ac!("AC-A1 AddMemoryRequest has intent_id field in web_server.rs", {
        let src = read_file("src/web_server.rs");
        assert!(
            src.contains("intent_id: Option<String>"),
            "AddMemoryRequest must have intent_id: Option<String>"
        );
        // Must be in the struct, not just in a closure
        assert!(
            src.contains("Adapter field") || src.contains("AcceptReceipt seam"),
            "intent_id field must have a doc comment explaining the adapter"
        );
    });

    // ── AC-A2 ─────────────────────────────────────────────────────────────────
    ac!("AC-A2 handle_add_memory returns 400 redirect when intent_id + Temporal", {
        let src = read_file("src/web_server.rs");
        assert!(
            src.contains("AcceptReceipt adapter") || src.contains("redirect"),
            "handle_add_memory must have adapter guard returning redirect advice"
        );
        assert!(
            src.contains("/intent/receipt") && src.contains("BAD_REQUEST"),
            "adapter guard must return BAD_REQUEST with /intent/receipt redirect"
        );
        assert!(
            src.contains("intent_id detected") || src.contains("intent_id={iid}"),
            "error message must name the intent_id it detected"
        );
    });

    // ── AC-A3 ─────────────────────────────────────────────────────────────────
    ac!("AC-A3 MCP add_memory routes to accept_receipt when intent_id present", {
        let src = read_file("sdk/mcp/server.py");
        assert!(
            src.contains("add_memory_adapter") || src.contains("handle_accept_receipt"),
            "MCP handle_add_memory must route to handle_accept_receipt when intent_id present"
        );
        assert!(
            src.contains("args.get(\"intent_id\")"),
            "MCP adapter must check intent_id arg"
        );
    });

    // ── AC-A4 ─────────────────────────────────────────────────────────────────
    ac!("AC-A4 Python SDK add_memory routes to /intent/receipt when intent_id set", {
        let src = read_file("sdk/python/hipcortex/client.py");
        assert!(
            src.contains("intent_id: Optional[str] = None"),
            "client.py add_memory must have intent_id kwarg"
        );
        assert!(
            src.contains("/intent/receipt"),
            "client.py add_memory must post to /intent/receipt when intent_id set"
        );
        assert!(
            src.contains("add_memory_adapter"),
            "client.py must set sensor_path=add_memory_adapter to trace routing"
        );
    });

    // ── AC-A5 ─────────────────────────────────────────────────────────────────
    ac!("AC-A5 harness.md documents adapter routing (SDK/MCP + REST 400)", {
        let src = read_file("docs/harness.md");
        assert!(
            src.contains("auto-routes to /intent/receipt"),
            "harness.md must say SDK/MCP add_memory auto-routes to /intent/receipt"
        );
        assert!(
            src.contains("returns 400") || src.contains("400"),
            "harness.md must document that REST /memory/add + intent_id returns 400"
        );
        assert!(
            src.contains("no intent_id"),
            "harness.md must clarify /memory/add is for non-intent Temporal (no intent_id)"
        );
    });

    println!("\n=== Acceptance v2.5.0 (IG Ranking + add_memory Adapter): 10/10 passed ===");
}
