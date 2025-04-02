use std::num::NonZeroUsize;

pub struct OrderedResult<T> {
    pub result: T,
    pub indx: NonZeroUsize,
}

impl<T> PartialOrd for OrderedResult<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.indx.partial_cmp(&self.indx)
    }
}

impl<T> Ord for OrderedResult<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.indx.partial_cmp(&self.indx).unwrap()
    }
}

impl<T> Eq for OrderedResult<T> {}

impl<T> PartialEq for OrderedResult<T> {
    fn eq(&self, other: &Self) -> bool {
        other.indx == self.indx
    }
}
