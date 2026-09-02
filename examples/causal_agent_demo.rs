//! Causal intervention demo for agent harness (per analysis recs).
//! Shows real(ish) backdoor + counterfactual after recent impl improvements.
//! Run: cargo run --example causal_agent_demo

use hipcortex::world_model_enhanced::{InterventionQuery, WorldModelEnhanced};
use std::collections::HashMap;

fn main() {
    let wm = WorldModelEnhanced::new();

    // Build simple causal graph via WM (delegates to CausalGraph; adds nodes+edges)
    let _ = wm.add_causal_edge("treatment".into(), "outcome".into());
    let _ = wm.add_causal_edge("confounder".into(), "treatment".into());
    let _ = wm.add_causal_edge("confounder".into(), "outcome".into());

    // Intervention: do(treatment=1.0)
    let query = InterventionQuery {
        intervention_var: "treatment".to_string(),
        intervention_value: 1.0,
        conditioned_on: std::collections::HashMap::new(),
        outcome: "outcome".to_string(),
        intervention_label: None,
    };
    if let Ok(dist) = wm.causal_intervention(query) {
        println!("Intervention P(outcome | do(treatment=1)): {:?}", dist);
    }

    // Counterfactual
    let mut actual = HashMap::new();
    actual.insert("treatment".into(), 0.0);
    actual.insert("outcome".into(), 0.2);
    if let Ok(cf) = wm.counterfactual(actual, "treatment".into(), 1.0) {
        println!("Counterfactual outcome if treatment=1: {:?}", cf);
    }

    println!("Demo: substrate now supports better causal 'what if' without always calling LLM.");
}
