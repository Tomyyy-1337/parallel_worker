use std::cell::Cell;

pub trait CellUpdate<T> {
    fn modify<F: FnOnce(T) -> T>(&self, f: F);
}

impl<T: Copy> CellUpdate<T> for Cell<T> {
    fn modify<F: FnOnce(T) -> T>(&self, f: F) {
        let value = self.get();
        self.set(f(value));
    }
}