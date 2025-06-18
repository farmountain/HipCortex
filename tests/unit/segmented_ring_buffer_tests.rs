use hipcortex::segmented_buffer::SegmentedRingBuffer;
use std::time::Duration;

#[test]
fn invariants_hold_many_ops() {
    let mut buf = SegmentedRingBuffer::new(100, 10);
    for i in 0..1000 {
        buf.push_back(i);
        if i % 3 == 0 {
            buf.pop_front();
        }
    }
    for _ in 0..50 {
        buf.pop_front();
    }
    buf.assert_invariants();
}

#[test]
fn coalesce_after_retain() {
    let mut buf = SegmentedRingBuffer::new(10, 4);
    for i in 0..8 {
        buf.push_back(i);
    }
    buf.retain(|x| *x == 3 || *x == 7);
    assert_eq!(buf.len(), 2);
    buf.assert_invariants();
    // After retain segments should have coalesced into one
    assert_eq!(buf.flush_all_segments(), vec![vec![3, 7]]);
}

#[test]
fn flush_in_order() {
    let mut buf = SegmentedRingBuffer::new(10, 3);
    for i in 1..=5 {
        buf.push_back(i);
    }
    let first = buf.flush_front_segment().unwrap();
    assert_eq!(first, vec![1, 2, 3]);
    let rest = buf.flush_all_segments();
    assert_eq!(rest, vec![vec![4, 5]]);
}

#[test]
fn insert_rate_and_occupancy() {
    let mut buf = SegmentedRingBuffer::new(5, 2);
    buf.push_back(1);
    std::thread::sleep(Duration::from_millis(10));
    buf.push_back(2);
    assert!(buf.get_mean_insert_rate() > 0.0);
    assert!((buf.get_occupancy_ratio() - 0.4).abs() < 1e-6);
}
