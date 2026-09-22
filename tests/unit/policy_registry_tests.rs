use hipcortex::payloads::PolicyPayload;
use hipcortex::policy_registry::{PolicyRecord, PolicyRegistry};
use uuid::Uuid;

fn make_policy(entity: Uuid, condition: &str, action: &str, priority: u32) -> PolicyRecord {
    PolicyRecord {
        id: Uuid::new_v4(),
        payload: PolicyPayload {
            entity_id: entity,
            trigger_condition: condition.into(),
            action_fn: action.into(),
            priority,
            active: true,
            law_id: None,
        },
    }
}

#[test]
fn active_for_entity_returns_sorted_by_priority_desc() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(make_policy(entity, "x > 1", "brake(0.5)", 5));
    reg.register_record(make_policy(entity, "x > 2", "brake(1.0)", 10));
    let policies = reg.active_for_entity(entity);
    assert_eq!(policies.len(), 2);
    assert_eq!(policies[0].payload.priority, 10, "highest priority must be first");
}

#[test]
fn inactive_policies_excluded_from_active() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    let mut pol = make_policy(entity, "x > 1", "brake(0.5)", 5);
    pol.payload.active = false;
    reg.register_record(pol);
    assert!(reg.active_for_entity(entity).is_empty());
}

#[test]
fn deactivate_removes_from_active() {
    let entity = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    let pol = make_policy(entity, "x > 1", "brake(0.5)", 5);
    let pol_id = pol.id;
    reg.register_record(pol);
    assert_eq!(reg.active_for_entity(entity).len(), 1);
    let removed = reg.deactivate(pol_id);
    assert!(removed, "deactivate must return true for known policy");
    assert!(reg.active_for_entity(entity).is_empty());
}

#[test]
fn different_entities_are_isolated() {
    let e1 = Uuid::new_v4();
    let e2 = Uuid::new_v4();
    let mut reg = PolicyRegistry::new();
    reg.register_record(make_policy(e1, "x > 1", "act_a", 1));
    assert!(reg.active_for_entity(e2).is_empty(), "e2 should see no policies");
    assert_eq!(reg.active_for_entity(e1).len(), 1);
}
