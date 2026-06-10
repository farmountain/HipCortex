// Chain-of-Thought: HealthReporter trait provides a uniform interface for all
// storage modules to report their health metrics to the SelfModel. Each module
// tracks latency, error count, and resource usage internally, and the trait
// exposes these as ModuleHealth for aggregation by HealthAggregator.
//
// Design: minimal surface — two methods. `report_health()` returns current
// snapshot. `health_module_name()` identifies the module in the aggregator.
// The trait is object-safe so modules can be collected into Vec<&dyn HealthReporter>.

use crate::self_model::ModuleHealth;

/// Trait for modules that can report health metrics to the Self-Model.
pub trait HealthReporter {
    /// Return a snapshot of current module health metrics.
    /// Called periodically by HealthAggregator or on-demand via health endpoints.
    fn report_health(&self) -> ModuleHealth;

    /// Human-readable module name for identification in aggregated reports.
    fn health_module_name(&self) -> &'static str;
}
