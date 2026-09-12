//! CapabilityCatalog — the single declared authority for "what can this server do".
//!
//! ## Chain-of-Thought
//!
//! 1. H8's root cause was not that a list existed, but that **four** lists existed
//!    with no cross-check: the live router, `/openapi.json`, the bootstrap
//!    capability registry (`bin/webserver.rs`), and the query list in
//!    `handle_self_capabilities`. They had already diverged — and the bootstrap
//!    list was *disjoint* from every vocabulary the decision engine is asked
//!    about, so `SelfModel::can_execute` rejected every op it was ever given
//!    ("Capability 'x' not registered"), making the registry inert.
//! 2. The fix is therefore not "maintain the list more carefully" but "hold no
//!    names of our own". This module projects capabilities out of the two
//!    authorities that already own them:
//!      - the router, via [`crate::openapi_spec::ROUTE_TABLE`] — every route
//!        yields exactly one capability, so removing a route removes its
//!        capability and adding one registers it;
//!      - [`crate::action_registry::ALL_OPS`] — the agent-op vocabulary that
//!        `can_execute` is actually queried with.
//! 3. Because nothing here is hand-listed, there is nothing left to drift. The
//!    `capability_catalog_sit` suite is the guard: it asserts the projection is
//!    injective and invertible, that the union covers the queried vocabulary,
//!    and that a newly declared route appears as a capability with no other
//!    edit. Falsification of that guard is part of the acceptance evidence.
//!
//! Precedent honoured: a component that owns an operation registers it itself
//! (`TemporalIndexer::with_self_model` → `temporal_insert`). This module covers
//! only what has no owning component — the route surface and the agent-op
//! vocabulary.

use crate::self_model::{CapabilityDescriptor, SelfModel};

/// A capability and the reason it is in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CapabilityOrigin {
    /// Capability name handed to [`SelfModel::register_capability`].
    pub name: String,
    /// `"route"`, `"agent-op"`, or both joined by `+`, in that order.
    pub origin: String,
}

/// Every capability the bootstrap must register, deduplicated and sorted.
///
/// Sourced entirely from the router (`ROUTE_TABLE`) and the agent-op vocabulary
/// (`ALL_OPS`). If this returns a name, some authority already declares it.
pub fn declared_capabilities() -> Vec<CapabilityOrigin> {
    let mut names: Vec<(String, bool, bool)> = Vec::new();

    for name in crate::openapi_spec::route_capability_names() {
        names.push((name, true, false));
    }
    for op in crate::action_registry::ALL_OPS {
        match names.iter_mut().find(|(n, _, _)| n == op) {
            Some(entry) => entry.2 = true,
            None => names.push(((*op).to_string(), false, true)),
        }
    }

    names.sort_by(|a, b| a.0.cmp(&b.0));
    names
        .into_iter()
        .map(|(name, is_route, is_op)| CapabilityOrigin {
            name,
            origin: match (is_route, is_op) {
                (true, true) => "route+agent-op".to_string(),
                (true, false) => "route".to_string(),
                (false, true) => "agent-op".to_string(),
                (false, false) => unreachable!("a name is always sourced from at least one authority"),
            },
        })
        .collect()
}

/// Convenience view of [`declared_capabilities`] when only the names matter.
pub fn declared_capability_names() -> Vec<String> {
    declared_capabilities().into_iter().map(|c| c.name).collect()
}

/// Why `name` is in the catalog: `"route"`, `"agent-op"`, `"route+agent-op"`, or
/// `"component"` for names registered by the component that owns the operation
/// (`TemporalIndexer` → `temporal_insert`) rather than by this catalog.
pub fn origin_of(name: &str) -> &'static str {
    match declared_capabilities().into_iter().find(|c| c.name == name) {
        Some(entry) => match entry.origin.as_str() {
            "route" => "route",
            "agent-op" => "agent-op",
            _ => "route+agent-op",
        },
        None => "component",
    }
}

/// Register every declared capability on `sm`.
///
/// Returns the names it registered. Already-registered names (a component that
/// owns the op got there first, e.g. `temporal_insert`) are left untouched, so
/// this is safe to call on a `SelfModel` that has been attached to components.
pub fn register_declared_capabilities(sm: &SelfModel) -> Vec<String> {
    let mut registered = Vec::new();
    for CapabilityOrigin { name, origin } in declared_capabilities() {
        let descriptor = CapabilityDescriptor {
            name: name.clone(),
            description: format!("HipCortex {} (origin: {})", name, origin),
            required_cpu_percent: 5.0,
            required_memory_mb: 50.0,
            limitations: vec![],
        };
        // `Err` here means "already registered by its owning component", which is
        // the desired end state, not a failure — hence no error propagation.
        if sm.register_capability(descriptor).is_ok() {
            registered.push(name);
        }
    }
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_non_empty_and_sorted() {
        let caps = declared_capability_names();
        assert!(!caps.is_empty(), "a server with no capabilities serves nothing");
        let mut sorted = caps.clone();
        sorted.sort();
        assert_eq!(caps, sorted, "catalog must be deterministically ordered");
    }

    #[test]
    fn every_agent_op_is_a_declared_capability() {
        let caps = declared_capability_names();
        for op in crate::action_registry::ALL_OPS {
            assert!(
                caps.iter().any(|c| c == op),
                "agent op '{}' is queried via can_execute but is not in the catalog — \
                 the decision engine will reject it as 'not registered'",
                op
            );
        }
    }

    #[test]
    fn route_capabilities_report_route_origin() {
        let caps = declared_capabilities();
        let add = caps
            .iter()
            .find(|c| c.name == crate::openapi_spec::capability_name("POST", "/memory/add"))
            .expect("POST /memory/add must yield a capability");
        assert_eq!(add.origin, "route");
    }
}
