use std::collections::VecDeque;

pub struct ResultBuffer<T> {
    buffer: VecDeque<Option<T>>,
    offset: usize,
}

impl <T> ResultBuffer<T> {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            offset: 0,
        }
    }

    pub fn insert(&mut self, item: T, index: usize) {
        if index >= self.buffer.len() {
            self.buffer.resize_with(index + 1, || None);
        }
        self.buffer[index - self.offset] = Some(item);
    }

    pub fn get(&mut self) -> Option<T> {
        match self.buffer.front() {
            Some(Some(_)) => {
                self.offset += 1;
                self.buffer.pop_front().unwrap()
            }
            _ => return None,
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.buffer.front().and_then(|item| item.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_buffer() {
        let mut buffer = ResultBuffer::new();
        buffer.insert(1, 0);
        buffer.insert(2, 1);
        buffer.insert(3, 2);

        assert_eq!(buffer.get(), Some(1));
        assert_eq!(buffer.get(), Some(2));
        assert_eq!(buffer.get(), Some(3));
        assert_eq!(buffer.get(), None);
    }

    #[test]
    fn test_result_buffer_with_gaps() {
        let mut buffer = ResultBuffer::new();
        buffer.insert(1, 0);
        buffer.insert(3, 2); 
        buffer.insert(2, 1);

        assert_eq!(buffer.get(), Some(1));
        assert_eq!(buffer.get(), Some(2));
        assert_eq!(buffer.get(), Some(3));
        assert_eq!(buffer.get(), None);
    }

    #[test]
    fn test_result_buffer_with_empty_slots() {
        let mut buffer = ResultBuffer::new();
        buffer.insert(1, 0);
        buffer.insert(3, 2); 
        buffer.insert(2, 1);
        buffer.insert(5, 4); 
        buffer.insert(4, 3); 

        assert_eq!(buffer.get(), Some(1));
        assert_eq!(buffer.get(), Some(2));

        buffer.insert(7, 6);
        buffer.insert(6, 5);
        buffer.insert(9, 8);
        buffer.insert(8, 7);
        assert_eq!(buffer.get(), Some(3));
        buffer.insert(11, 10);
        buffer.insert(10, 9);
        buffer.insert(13, 12);
        buffer.insert(12, 11);
        assert_eq!(buffer.get(), Some(4));
        buffer.insert(14, 13);
        buffer.insert(16, 15);
        buffer.insert(15, 14);
        buffer.insert(17, 16);
        
        assert_eq!(buffer.get(), Some(5));
        assert_eq!(buffer.get(), Some(6));
        assert_eq!(buffer.get(), Some(7));
        assert_eq!(buffer.get(), Some(8));
        assert_eq!(buffer.get(), Some(9));
        assert_eq!(buffer.get(), Some(10));
        assert_eq!(buffer.get(), Some(11));
        assert_eq!(buffer.get(), Some(12));
        assert_eq!(buffer.get(), Some(13));
        assert_eq!(buffer.get(), Some(14));
        assert_eq!(buffer.get(), Some(15));
        assert_eq!(buffer.get(), Some(16));
        assert_eq!(buffer.get(), Some(17));
        
        assert_eq!(buffer.get(), None);
    }
}