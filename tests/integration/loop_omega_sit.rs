#[cfg(test)]
mod tests {
    use hipcortex::loop_engine::LoopEngine;
    use hipcortex::topological_memory::CausalTopoGraph;

    // AC-G3a: run_omega_loop advances iterations counter and returns Ok
    #[test]
    fn test_run_omega_loop_advances_iterations() {
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);

        let result = engine.run_omega_loop();
        assert!(result.is_ok(), "run_omega_loop must succeed: {:?}", result.err());
        assert!(
            engine.metrics.iterations >= 1,
            "run_omega_loop must advance iterations counter; got {}",
            engine.metrics.iterations
        );
    }

    // AC-G3a extended: two successive calls advance to >= 2 iterations
    #[test]
    fn test_run_omega_loop_increments_on_each_call() {
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);

        engine.run_omega_loop().expect("first call failed");
        engine.run_omega_loop().expect("second call failed");

        assert!(
            engine.metrics.iterations >= 2,
            "two calls must produce >= 2 iterations; got {}",
            engine.metrics.iterations
        );
    }
}
