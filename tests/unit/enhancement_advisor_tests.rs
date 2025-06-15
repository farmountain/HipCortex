use hipcortex::enhancement_advisor::{ComponentMetric, EnhancementAdvisor};

#[test]
fn basic_advisor_usage() {
    let mut adv = EnhancementAdvisor::new();
    let metrics = vec![
        ComponentMetric {
            name: "latency".into(),
            value: 0.4,
        },
        ComponentMetric {
            name: "error_rate".into(),
            value: 0.4,
        },
    ];
    adv.analyze("IntegrationLayer", &metrics);
    assert_eq!(adv.recommendations().len(), 1);
    assert!(adv.recommendations()[0].contains("IntegrationLayer"));
    adv.reset();
    assert!(adv.recommendations().is_empty());
}
