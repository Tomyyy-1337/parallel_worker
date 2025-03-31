use std::{cell::{Cell, RefCell}, collections::BinaryHeap};

use crate::{internal::CellUpdate, worker_traits::{WorkerInit, WorkerMethods}};

use super::BasicWorker;

struct OrderedResult<T> {
    result: T,
    indx: usize,
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

/// A worker that processes tasks in parallel using multiple worker threads.
/// The results are returned in same order as the tasks were added.
pub struct OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: BasicWorker<(usize, T), (usize, R)>,
    result_heap: RefCell<BinaryHeap<OrderedResult<R>>>,
    task_indx: Cell<usize>,
    result_indx: Cell<usize>,
}

impl<T, R> WorkerMethods<T, R> for OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&self, task: T) {
        self.inner.add_task((self.task_indx.get(), task));
        self.task_indx.modify(|i| i + 1);
    }

    fn add_tasks(&self, tasks: impl IntoIterator<Item = T>) {
        self.inner.add_tasks(tasks.into_iter().map(|t| {
            let task = (self.task_indx.get(), t);
            self.task_indx.modify(|i| i + 1);
            task
        }));
    }

    /// Clear the task queue. Ongoing tasks will not be canceled.
    /// Results of ongoing and already completed tasks will remain unaffected.
    fn cancel_tasks(&self) {
        self.inner.cancel_tasks();
    }

    fn get(&self) -> Option<R> {
       Self::get_in_order(&self, |inner| inner.get())
    }

    fn get_blocking(&self) -> Option<R> {
        Self::get_in_order(&self, |inner| inner.get_blocking())
    }

    fn pending_tasks(&self) -> usize {
        self.inner.pending_tasks()
    }
}

impl<T, R> OrderedWorker<T, R> 
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn get_in_order(&self, get_function: impl Fn(&BasicWorker<(usize, T), (usize, R)>) -> Option<(usize, R)>) -> Option<R> {
        let mut result_heap = self.result_heap.borrow_mut();
        if let Some(&OrderedResult{indx, ..}) = result_heap.peek() {
            if indx == self.result_indx.get() {
                self.result_indx.modify(|i| i + 1);
                let result = result_heap.pop().unwrap().result;
                return Some(result);
            }
        }
        while let Some((indx ,result)) = get_function(&self.inner) {
            if indx == self.result_indx.get() {
                self.result_indx.modify(|i| i + 1);
                return Some(result);
            }
            result_heap.push(OrderedResult { result, indx });   
        }
        None
    }
}

impl <T, R, F> WorkerInit<T, R, F> for OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Copy + 'static,
{
    fn with_num_threads(num_worker_threads: usize, worker_function: F) -> Self {
        let inner = BasicWorker::with_num_threads(num_worker_threads, move |(indx, task)| {
            let result = worker_function(task);
            (indx, result)
        });

        Self {
            inner,
            result_heap: RefCell::new(BinaryHeap::new()),
            task_indx: Cell::new(0),
            result_indx: Cell::new(0),
        }
    }
}

