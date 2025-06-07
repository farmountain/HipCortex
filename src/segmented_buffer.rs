use std::collections::VecDeque;

/// Segmented ring buffer that grows by segment for cheap push/pop
pub struct SegmentedRingBuffer<T> {
    segment_size: usize,
    capacity: usize,
    segments: VecDeque<VecDeque<T>>,
    len: usize,
}

impl<T> SegmentedRingBuffer<T> {
    pub fn new(capacity: usize, segment_size: usize) -> Self {
        Self {
            segment_size: segment_size.max(1),
            capacity,
            segments: VecDeque::new(),
            len: 0,
        }
    }

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
                return Some(item);
            }
        }
        None
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
