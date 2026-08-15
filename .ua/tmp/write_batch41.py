import json

data = {
  "nodes": [
    {
      "id": "file:src/modules/world_model_enhanced/causal.rs",
      "type": "file",
      "name": "causal.rs",
      "filePath": "src/modules/world_model_enhanced/causal.rs",
      "summary": "Implements CausalGraph, a directed acyclic graph supporting do-calculus interventions, backdoor adjustment, SCM counterfactuals, and empirical distribution tracking. Recently extended with auto_populate_from_transitions() that seeds causal distributions from a Dirichlet transition model and an intervention_label field on InterventionQuery.",
      "tags": ["causal-inference", "graph", "world-model", "service", "utility"],
      "complexity": "complex",
      "languageNotes": "Uses HashMap-keyed empirical distributions keyed by string condition/outcome pairs; backdoor adjustment iterates cartesian products over confounder states computed at runtime."
    },
    {
      "id": "class:src/modules/world_model_enhanced/causal.rs:CausalGraph",
      "type": "class",
      "name": "CausalGraph",
      "filePath": "src/modules/world_model_enhanced/causal.rs",
      "lineRange": [53, 717],
      "summary": "Core DAG structure storing nodes, directed edges, edge strengths, and conditional distributions; exposes graph traversal (BFS path-finding, DFS all-paths, cycle detection) alongside causal inference methods (backdoor-adjusted intervention, SCM counterfactual, empirical intervention).",
      "tags": ["causal-inference", "dag", "do-calculus", "graph-algorithm"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/causal.rs:compute_intervention",
      "type": "function",
      "name": "compute_intervention",
      "filePath": "src/modules/world_model_enhanced/causal.rs",
      "lineRange": [259, 355],
      "summary": "Applies the backdoor adjustment formula to estimate the causal effect of setting an intervention variable to a value on an outcome variable, summing over confounder states weighted by their prior probabilities.",
      "tags": ["do-calculus", "backdoor-adjustment", "causal-inference"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/causal.rs:auto_populate_from_transitions",
      "type": "function",
      "name": "auto_populate_from_transitions",
      "filePath": "src/modules/world_model_enhanced/causal.rs",
      "lineRange": [407, 461],
      "summary": "Bootstraps the causal graph empirical distributions from a Dirichlet-Multinomial transition model: adds state/action/next_state nodes and edges, computes a state prior from observation counts, and derives marginal next-state distributions via model.predict for each state-action pair.",
      "tags": ["causal-inference", "transition-model", "distribution-seeding"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/causal.rs:compute_scm_counterfactual",
      "type": "function",
      "name": "compute_scm_counterfactual",
      "filePath": "src/modules/world_model_enhanced/causal.rs",
      "lineRange": [577, 650],
      "summary": "Performs structural causal model counterfactual inference: abduces noise terms from observed state, applies the intervention by fixing a variable, and re-predicts remaining variables in topological order using edge strengths.",
      "tags": ["counterfactual", "scm", "causal-inference"],
      "complexity": "complex"
    },
    {
      "id": "file:src/modules/world_model_enhanced/entity.rs",
      "type": "file",
      "name": "entity.rs",
      "filePath": "src/modules/world_model_enhanced/entity.rs",
      "summary": "Implements EntityTracker, a Kalman filter for tracking entity state vectors over time, including anomaly detection via Mahalanobis distance. Extended with EntityConfig allowing a custom state-transition matrix F and a with_config constructor.",
      "tags": ["kalman-filter", "entity-tracking", "anomaly-detection", "world-model"],
      "complexity": "complex",
      "languageNotes": "Matrix math (transpose, multiply, add, subtract, Gauss-Jordan inverse) is implemented inline without external linear algebra crates, keeping the minimal petgraph_backend build dependency-free."
    },
    {
      "id": "class:src/modules/world_model_enhanced/entity.rs:EntityTracker",
      "type": "class",
      "name": "EntityTracker",
      "filePath": "src/modules/world_model_enhanced/entity.rs",
      "lineRange": [44, 221],
      "summary": "Kalman filter tracker holding state mean vector, covariance matrix, transition/observation/noise matrices, and an anomaly list; supports multi-step prediction, Bayesian state updates, Mahalanobis anomaly scoring, and staleness checks.",
      "tags": ["kalman-filter", "state-estimation", "anomaly-detection"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/entity.rs:update",
      "type": "function",
      "name": "update",
      "filePath": "src/modules/world_model_enhanced/entity.rs",
      "lineRange": [122, 201],
      "summary": "Executes the Kalman filter update step: computes innovation residual, Kalman gain, updated state mean and covariance, detects anomalies via Mahalanobis distance, and applies Joseph-form covariance stabilization.",
      "tags": ["kalman-filter", "bayesian-update", "anomaly-detection"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/entity.rs:matrix_inverse",
      "type": "function",
      "name": "matrix_inverse",
      "filePath": "src/modules/world_model_enhanced/entity.rs",
      "lineRange": [322, 381],
      "summary": "Computes the inverse of a square matrix via Gauss-Jordan elimination with partial pivoting; returns Err for singular or non-square matrices.",
      "tags": ["linear-algebra", "matrix-inversion", "utility"],
      "complexity": "complex"
    },
    {
      "id": "file:src/modules/world_model_enhanced/mod.rs",
      "type": "file",
      "name": "mod.rs",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "summary": "Defines WorldModelEnhanced, the top-level coordinator for the enhanced world model: wraps a Dirichlet transition model, Kalman entity map, causal DAG, learned predictors, and uncertainty estimator all behind Arc<RwLock<...>>, and exposes transition observation, entity tracking, causal intervention, MCTS planning, and rollout methods. New sync_causal_distributions() bridges the transition model into the causal graph.",
      "tags": ["world-model", "coordinator", "service", "mcts", "causal-inference"],
      "complexity": "complex",
      "languageNotes": "All subsystems are Arc<RwLock<T>>-wrapped for concurrent access; the consistent read/write-lock pattern across every method enforces a safe shared-state contract."
    },
    {
      "id": "class:src/modules/world_model_enhanced/mod.rs:WorldModelEnhanced",
      "type": "class",
      "name": "WorldModelEnhanced",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [96, 824],
      "summary": "Facade class combining Dirichlet-Multinomial transition learning, Kalman entity tracking, causal graph inference, learned predictor ensemble, uncertainty quantification, and MCTS-based planning into a single concurrency-safe coordinator.",
      "tags": ["world-model", "coordinator", "planning", "causal-inference"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:sync_causal_distributions",
      "type": "function",
      "name": "sync_causal_distributions",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [638, 642],
      "summary": "Reads the current transition model and calls auto_populate_from_transitions on the causal graph, keeping empirical causal distributions in sync with observed state-action-nextstate counts.",
      "tags": ["causal-inference", "sync", "world-model"],
      "complexity": "simple"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:mcts_best_action_goal",
      "type": "function",
      "name": "mcts_best_action_goal",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [503, 539],
      "summary": "Runs Monte Carlo Tree Search to select the best action from a root state toward a goal state, using UCB1 exploration and the Dirichlet transition model for simulated rollouts.",
      "tags": ["mcts", "planning", "world-model"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:causal_intervention",
      "type": "function",
      "name": "causal_intervention",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [318, 362],
      "summary": "Public facade for causal intervention queries: normalises the intervention value, routes to empirical or structural computation on the causal graph, and fills in a default outcome if the graph returns empty.",
      "tags": ["causal-inference", "do-calculus", "api-handler"],
      "complexity": "complex"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:rollout_dirichlet",
      "type": "function",
      "name": "rollout_dirichlet",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [430, 465],
      "summary": "Simulates a multi-step trajectory through the Dirichlet transition model by greedily selecting the most probable next state at each step, returning the final state and joint confidence.",
      "tags": ["rollout", "planning", "world-model"],
      "complexity": "moderate"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:save",
      "type": "function",
      "name": "save",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [687, 735],
      "summary": "Serialises the WorldModelEnhanced state (transitions, entities, causal edges) to a directory of JSON files for persistence.",
      "tags": ["persistence", "serialization", "world-model"],
      "complexity": "moderate"
    },
    {
      "id": "function:src/modules/world_model_enhanced/mod.rs:load",
      "type": "function",
      "name": "load",
      "filePath": "src/modules/world_model_enhanced/mod.rs",
      "lineRange": [739, 818],
      "summary": "Deserialises WorldModelEnhanced from a directory of JSON files, reconstructing the transition model, entity trackers, and causal graph edges.",
      "tags": ["persistence", "deserialization", "world-model"],
      "complexity": "complex"
    }
  ],
  "edges": [
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "file:src/modules/world_model_enhanced/transition.rs", "type": "imports", "direction": "forward", "weight": 0.7},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "class:src/modules/world_model_enhanced/causal.rs:CausalGraph", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "class:src/modules/world_model_enhanced/causal.rs:CausalGraph", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:compute_intervention", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:compute_intervention", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:auto_populate_from_transitions", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:auto_populate_from_transitions", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:compute_scm_counterfactual", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/causal.rs", "target": "function:src/modules/world_model_enhanced/causal.rs:compute_scm_counterfactual", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/entity.rs", "target": "class:src/modules/world_model_enhanced/entity.rs:EntityTracker", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/entity.rs", "target": "class:src/modules/world_model_enhanced/entity.rs:EntityTracker", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/entity.rs", "target": "function:src/modules/world_model_enhanced/entity.rs:update", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/entity.rs", "target": "function:src/modules/world_model_enhanced/entity.rs:update", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/entity.rs", "target": "function:src/modules/world_model_enhanced/entity.rs:matrix_inverse", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "file:src/modules/world_model_enhanced/causal.rs", "type": "imports", "direction": "forward", "weight": 0.7},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "file:src/modules/world_model_enhanced/entity.rs", "type": "imports", "direction": "forward", "weight": 0.7},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "file:src/modules/world_model_enhanced/transition.rs", "type": "imports", "direction": "forward", "weight": 0.7},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "file:src/modules/world_model_enhanced/uncertainty.rs", "type": "imports", "direction": "forward", "weight": 0.7},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "class:src/modules/world_model_enhanced/mod.rs:WorldModelEnhanced", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "class:src/modules/world_model_enhanced/mod.rs:WorldModelEnhanced", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:sync_causal_distributions", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:sync_causal_distributions", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:mcts_best_action_goal", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:mcts_best_action_goal", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:causal_intervention", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:causal_intervention", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:rollout_dirichlet", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:rollout_dirichlet", "type": "exports", "direction": "forward", "weight": 0.8},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:save", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "file:src/modules/world_model_enhanced/mod.rs", "target": "function:src/modules/world_model_enhanced/mod.rs:load", "type": "contains", "direction": "forward", "weight": 1.0},
    {"source": "function:src/modules/world_model_enhanced/causal.rs:auto_populate_from_transitions", "target": "file:src/modules/world_model_enhanced/transition.rs", "type": "calls", "direction": "forward", "weight": 0.8},
    {"source": "function:src/modules/world_model_enhanced/mod.rs:sync_causal_distributions", "target": "function:src/modules/world_model_enhanced/causal.rs:auto_populate_from_transitions", "type": "calls", "direction": "forward", "weight": 0.8},
    {"source": "function:src/modules/world_model_enhanced/mod.rs:causal_intervention", "target": "function:src/modules/world_model_enhanced/causal.rs:compute_intervention", "type": "calls", "direction": "forward", "weight": 0.8}
  ]
}

with open("D:/all_projects/hipcortex/.ua/intermediate/batch-41.json", "w") as f:
    json.dump(data, f, indent=2)
print("OK")
