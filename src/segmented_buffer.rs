use std::collections::VecDeque;
use std::time::Instant;

/// Segmented ring buffer that grows by segment for cheap push/pop
pub struct SegmentedRingBuffer<T> {
    segment_size: usize,
    capacity: usize,
    segments: VecDeque<VecDeque<T>>,
    len: usize,
    insert_times: VecDeque<Instant>,
}

impl<T> SegmentedRingBuffer<T> {
    const RATE_WINDOW: usize = 32;

    pub fn new(capacity: usize, segment_size: usize) -> Self {
        Self {
            segment_size: segment_size.max(1),
            capacity,
            segments: VecDeque::new(),
            len: 0,
            insert_times: VecDeque::with_capacity(Self::RATE_WINDOW),
        }
    }

    /// Append an item to the buffer, evicting the oldest if full.
    pub fn push_back(&mut self, item: T) {
        if self.len == self.capacity {
            self.pop_front();
        }
        if self
            .segments
            .back()
            .map_or(true, |seg| seg.len() == self.segment_size)
        {
            self.segments
                .push_back(VecDeque::with_capacity(self.segment_size));
        }
        self.segments.back_mut().unwrap().push_back(item);
        self.len += 1;
        let now = Instant::now();
        self.insert_times.push_back(now);
        if self.insert_times.len() > Self::RATE_WINDOW {
            self.insert_times.pop_front();
        }
        self.coalesce_segments();
        self.assert_invariants();
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if let Some(seg) = self.segments.front_mut() {
            let item = seg.pop_front();
            if item.is_some() {
                self.len -= 1;
            }
            if seg.is_empty() {
                self.segments.pop_front();
            }
            self.coalesce_segments();
            self.assert_invariants();
            item
        } else {
            None
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        for seg in self.segments.iter_mut() {
            seg.retain(|item| f(item));
        }
        while let Some(front) = self.segments.front() {
            if front.is_empty() {
                self.segments.pop_front();
            } else {
                break;
            }
        }
        self.len = self.segments.iter().map(|s| s.len()).sum();
        if self.len > self.capacity {
            while self.len > self.capacity {
                self.pop_front();
            }
        }
        self.coalesce_segments();
        self.assert_invariants();
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.segments.iter().flat_map(|s| s.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.segments.iter_mut().flat_map(|s| s.iter_mut())
    }

    pub fn position<F>(&self, mut f: F) -> Option<(usize, usize)>
    where
        F: FnMut(&T) -> bool,
    {
        for (seg_i, seg) in self.segments.iter().enumerate() {
            if let Some(pos) = seg.iter().position(|item| f(item)) {
                return Some((seg_i, pos));
            }
        }
        None
    }

    pub fn remove_at(&mut self, seg_idx: usize, pos: usize) -> Option<T> {
        if let Some(seg) = self.segments.get_mut(seg_idx) {
            if let Some(item) = seg.remove(pos) {
                self.len -= 1;
                if seg.is_empty() {
                    self.segments.remove(seg_idx);
                }
                self.coalesce_segments();
                self.assert_invariants();
                return Some(item);
            }
        }
        None
    }

    /// Ensure mathematical invariants of the ring buffer hold.
    pub fn assert_invariants(&self) {
        let total: usize = self.segments.iter().map(|s| s.len()).sum();
        assert_eq!(total, self.len, "length mismatch");
        assert!(self.len <= self.capacity, "len > capacity");
        let max_segments = (self.capacity + self.segment_size - 1) / self.segment_size;
        assert!(self.segments.len() <= max_segments, "too many segments");
    }

    /// Merge sparse neighbor segments and compress to keep segment count small.
    pub fn coalesce_segments(&mut self) {
        let mut i = 0;
        while i + 1 < self.segments.len() {
            let len_a = self.segments[i].len();
            let len_b = self.segments[i + 1].len();
            let threshold = self.segment_size / 2;
            if len_a < threshold && len_b < threshold && len_a + len_b <= self.segment_size {
                let other = self.segments.remove(i + 1).unwrap();
                self.segments[i].extend(other);
            } else {
                // move items forward to fill segment i
                while self.segments[i].len() < self.segment_size && !self.segments[i + 1].is_empty()
                {
                    if let Some(item) = self.segments[i + 1].pop_front() {
                        self.segments[i].push_back(item);
                    }
                }
                if self.segments[i + 1].is_empty() {
                    self.segments.remove(i + 1);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// Remove and return the front-most segment for persistence.
    pub fn flush_front_segment(&mut self) -> Option<Vec<T>> {
        self.segments.pop_front().map(|seg| {
            self.len -= seg.len();
            self.assert_invariants();
            seg.into_iter().collect()
        })
    }

    /// Dump all segments in order, clearing the buffer.
    pub fn flush_all_segments(&mut self) -> Vec<Vec<T>> {
        let mut out = Vec::new();
        while let Some(seg) = self.segments.pop_front() {
            self.len -= seg.len();
            out.push(seg.into_iter().collect());
        }
        self.assert_invariants();
        out
    }

    /// Average inserts per second over the rolling window.
    pub fn get_mean_insert_rate(&self) -> f32 {
        if self.insert_times.len() < 2 {
            return 0.0;
        }
        let start = self.insert_times.front().unwrap();
        let end = self.insert_times.back().unwrap();
        let dur = *end - *start;
        let secs = dur.as_secs_f32();
        if secs == 0.0 {
            return self.insert_times.len() as f32;
        }
        (self.insert_times.len() - 1) as f32 / secs
    }

    /// Ratio of current length to total capacity.
    pub fn get_occupancy_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.len as f32 / self.capacity as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop() {
        let mut buf = SegmentedRingBuffer::new(3, 2);
        buf.push_back(1);
        buf.push_back(2);
        buf.push_back(3);
        assert_eq!(buf.len(), 3);
        buf.push_back(4);
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.pop_front(), Some(2));
        assert_eq!(buf.pop_front(), Some(3));
        assert_eq!(buf.pop_front(), Some(4));
        assert_eq!(buf.pop_front(), None);
    }
}
