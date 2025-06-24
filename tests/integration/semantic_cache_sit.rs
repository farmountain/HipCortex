use hipcortex::semantic_cache::SemanticCache;

#[test]
fn cache_hits_and_eviction() {
    let mut cache = SemanticCache::new(2);
    cache.put_embedding("a".into(), vec![1.0, 0.0]);
    cache.put_embedding("b".into(), vec![0.0, 1.0]);
    // access a so b becomes least recently used
    cache.get_nearest(&[1.0, 0.0]);
    cache.put_embedding("c".into(), vec![0.5, 0.5]);
    assert_eq!(cache.len(), 2);
    let res = cache.get_nearest(&[0.0, 1.0]).unwrap();
    assert_ne!(res.0, "b");
}
