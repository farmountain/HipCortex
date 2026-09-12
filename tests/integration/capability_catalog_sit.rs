#![cfg(feature = "web-server")]
//! H8 / WP7 — the capability registry must be a *projection* of the router and
//! the agent-op vocabulary, never a list of its own.
//!
//! ## What was actually broken
//!
//! The spec described H8 as "capability registry is a 4th hand-maintained list".
//! Reading the code showed something sharper: there were **three** disjoint
//! vocabularies holding capability names —
//!
//! * `bin/webserver.rs` registered 15 names (`add_memory`, `ingest`, …),
//! * `handle_self_capabilities` queried 8 of those same names,
//! * `action_registry::ALL_OPS` held 12 *different* names (`store_memory`,
//!   `react_loop`, …) — and these are the names `can_execute` is actually asked
//!   about, by `list_authorized` and by `list_authorized_world_model` behind
//!   `GET /v1/actions/authorized-wm`.
//!
//! The intersection of the registered set and the queried set was **empty**, and
//! `SelfModel::can_execute` rejects unregistered ops with
//! `Capability '{}' not registered`. So `GET /v1/actions/authorized-wm` was
//! structurally guaranteed to answer `{"authorized":[]}` no matter how much the
//! world model had learned. That is the defect these tests lock down: not drift,
//! but a registry that could never be consulted.
//!
//! Run: `cargo test --no-default-features --features "petgraph_backend,web-server" \
//!              --test integration_suite capability_catalog`

use hipcortex::capability_catalog;
use hipcortex::openapi_spec::{capability_name, capability_route, route_capability_names, ROUTE_TABLE};
use hipcortex::self_model::SelfModel;
use std::sync::Arc;

// ── Projection invariants ────────────────────────────────────────────────────

/// The projection must be injective: two distinct routes may not collapse onto
/// one capability. This is the guard the doc comment on `capability_name`
/// promises, and it is the reason a future `/a/b` + `/a-b` pair fails loudly
/// instead of silently merging.
#[test]
fn capability_projection_is_injective() {
    let mut seen: std::collections::HashMap<String, (&str, &str)> = std::collections::HashMap::new();
    for (method, path) in ROUTE_TABLE {
        let name = capability_name(method, path);
        if let Some((other_method, other_path)) = seen.insert(name.clone(), (method, path)) {
            panic!(
                "routes '{} {}' and '{} {}' both project to capability '{}' — \
                 two capabilities would merge into one and the registry would \
                 under-report the served surface",
                other_method, other_path, method, path, name
            );
        }
    }
}

/// The projection must be invertible: every capability resolves back to exactly
/// the route it came from. Without this, a capability could exist with no route
/// behind it — which is the "advertises what it cannot do" failure the H1 work
/// removed from the OpenAPI document.
#[test]
fn every_capability_inverts_to_its_own_route() {
    for (method, path) in ROUTE_TABLE {
        let name = capability_name(method, path);
        match capability_route(&name) {
            Some((found_method, found_path)) => assert_eq!(
                (found_method, found_path),
                (*method, *path),
                "capability '{}' inverted to the wrong route",
                name
            ),
            None => panic!(
                "capability '{}' (from '{} {}') has no route — a capability with \
                 no route is exactly the drift H8 removed",
                name, method, path
            ),
        }
    }
}

/// No capability may be fabricated: every name the catalog declares either comes
/// from a declared route or from `ALL_OPS`. This is what "holds no names of its
/// own" means operationally.
#[test]
fn catalog_names_come_only_from_its_two_authorities() {
    let route_names: std::collections::HashSet<String> =
        route_capability_names().into_iter().collect();
    let op_names: std::collections::HashSet<String> = hipcortex::action_registry::ALL_OPS
        .iter()
        .map(|s| s.to_string())
        .collect();

    for entry in capability_catalog::declared_capabilities() {
        assert!(
            route_names.contains(&entry.name) || op_names.contains(&entry.name),
            "capability '{}' (origin '{}') is in neither the router nor ALL_OPS — \
             the catalog has grown a list of its own",
            entry.name,
            entry.origin
        );
        // Origin must match membership, so the reported provenance is not a lie.
        let from_route = route_names.contains(&entry.name);
        let from_op = op_names.contains(&entry.name);
        let expected = match (from_route, from_op) {
            (true, true) => "route+agent-op",
            (true, false) => "route",
            (false, true) => "agent-op",
            (false, false) => unreachable!(),
        };
        assert_eq!(entry.origin, expected, "wrong origin reported for '{}'", entry.name);
    }
}

/// Cardinality identity: catalog size must equal the size implied by its two
/// sources. A hand-written list cannot satisfy this for an arbitrary
/// `ROUTE_TABLE`, so this test is what makes "no literal list remains" checkable.
#[test]
fn catalog_size_is_implied_by_its_sources() {
    let routes = route_capability_names();
    let route_set: std::collections::HashSet<&String> = routes.iter().collect();
    let extra_ops = hipcortex::action_registry::ALL_OPS
        .iter()
        .filter(|op| !route_set.contains(&op.to_string()))
        .count();
    assert_eq!(
        capability_catalog::declared_capability_names().len(),
        routes.len() + extra_ops,
        "catalog size does not match |routes| + |ALL_OPS \\ routes| — the catalog \
         is not a pure projection of its two authorities"
    );
}

// ── The inert-registry regression ────────────────────────────────────────────

/// Every op the decision engine is asked about must be registered, or
/// `can_execute` rejects it before it evaluates anything.
#[test]
fn registration_makes_every_queried_op_decidable() {
    let sm = Arc::new(SelfModel::new());
    capability_catalog::register_declared_capabilities(&sm);

    let mut queried: Vec<&str> = hipcortex::action_registry::ALL_OPS.to_vec();
    for c in hipcortex::action_registry::WM_CONSTRAINTS {
        if !queried.contains(&c.op) {
            queried.push(c.op);
        }
    }

    for op in queried {
        let decision = sm
            .can_execute(op, hipcortex::self_model::DecisionContext::default_context())
            .expect("can_execute must not error for a registered op");
        assert!(
            !decision.rationale.contains("not registered"),
            "op '{}' was rejected as unregistered. A capability that is never \
             registered cannot be approved, so every endpoint built on \
             can_execute is structurally pinned to an empty answer.",
            op
        );
    }
}

/// `GET /v1/actions/authorized-wm` is the live consumer of this registry. With
/// the world model seeded, the registry must let at least one WM op through the
/// "capability exists" gate — otherwise the endpoint is decorative.
///
/// This does not assert approval (the DecisionEngine may still decline on
/// resources or health); it asserts the op is *evaluated*, which is the part H8
/// fixed.
#[test]
fn world_model_ops_reach_the_decision_engine() {
    let sm = Arc::new(SelfModel::new());
    capability_catalog::register_declared_capabilities(&sm);

    for constraint in hipcortex::action_registry::WM_CONSTRAINTS {
        let decision = sm
            .can_execute(
                constraint.op,
                hipcortex::self_model::DecisionContext::default_context(),
            )
            .expect("can_execute must not error");
        assert!(
            !decision.rationale.contains("not registered"),
            "WM op '{}' never reaches the decision engine",
            constraint.op
        );
    }
}

/// A component that owns an operation registers it itself (`TemporalIndexer` →
/// `temporal_insert`). Bootstrapping the catalog must not fail or duplicate in
/// that case — `register_capability` returns `Err` on a duplicate, and an
/// `Err` there must not be mistaken for a hard failure.
#[test]
fn registration_is_idempotent_over_component_registered_ops() {
    let sm = Arc::new(SelfModel::new());
    let first = capability_catalog::register_declared_capabilities(&sm);
    let count_after_first = sm.capability_count();
    assert_eq!(first.len(), count_after_first);

    let second = capability_catalog::register_declared_capabilities(&sm);
    assert!(
        second.is_empty(),
        "re-registering reported {} new capabilities; the catalog is not idempotent",
        second.len()
    );
    assert_eq!(
        sm.capability_count(),
        count_after_first,
        "re-registering changed the registry size"
    );
}

/// Sanity: the projection is total. A route that produces an empty name would be
/// unregisterable and invisible.
#[test]
fn no_route_projects_to_an_empty_capability() {
    for (method, path) in ROUTE_TABLE {
        let name = capability_name(method, path);
        assert!(
            !name.is_empty(),
            "route '{} {}' projected to an empty capability name",
            method,
            path
        );
        assert!(
            name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "capability '{}' is not a safe identifier",
            name
        );
    }
}

/// `origin_of` is the value `GET /self/capabilities` reports. It must never
/// claim a component-origin for something the catalog declares, nor the reverse.
#[test]
fn origin_of_agrees_with_the_catalog() {
    for entry in capability_catalog::declared_capabilities() {
        assert_eq!(
            capability_catalog::origin_of(&entry.name),
            entry.origin,
            "origin_of disagrees with declared_capabilities for '{}'",
            entry.name
        );
    }
    assert_eq!(
        capability_catalog::origin_of("__definitely_not_a_declared_capability__"),
        "component",
        "an undeclared name must be attributed to a component, not to the catalog"
    );
}
