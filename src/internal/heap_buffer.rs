use std::{collections::BinaryHeap, num::NonZeroUsize};

use super::OrderedResult;

pub struct HeapBuffer<T> {
    result_heap: BinaryHeap<OrderedResult<T>>,
    current_indx: NonZeroUsize,
}

impl<T> HeapBuffer<T> {
    pub fn new() -> Self {
        Self {
            result_heap: BinaryHeap::new(),
            current_indx: NonZeroUsize::MIN,
        }
    }

    pub fn push(&mut self, elem: T, indx: NonZeroUsize) {
        self.result_heap.push(OrderedResult { result: elem, indx });
    }

    pub fn get(&mut self) -> Option<T> {
        if let Some(&OrderedResult { indx, .. }) = self.result_heap.peek() {
            if indx == self.current_indx {
                self.current_indx = self.current_indx.saturating_add(1);
                return Some(self.result_heap.pop().unwrap().result);
            }
        }
        None
    }

    pub fn current_indx(&self) -> NonZeroUsize {
        self.current_indx
    }

    pub fn skip(&mut self) {
        self.current_indx = self.current_indx.saturating_add(1);
    }
}
