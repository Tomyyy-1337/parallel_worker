use std::collections::BinaryHeap;

use crate::{internal::OrderedResult, worker_traits::{WorkerInit, WorkerMethods}};

use super::BasicWorker;

/// A worker that processes tasks in parallel using multiple worker threads.
/// The results are returned in same order as the tasks were added.
pub struct OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: BasicWorker<(usize, T), (usize, R)>,
    result_heap: BinaryHeap<OrderedResult<R>>,
    task_indx: usize,
    result_indx: usize,
}

impl<T, R> WorkerMethods<T, R> for OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&mut self, task: T) {
        self.inner.add_task((self.task_indx, task));
        self.task_indx += 1;
    }

    fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>) {
        self.inner.add_tasks(tasks.into_iter().map(|t| {
            let task = (self.task_indx, t);
            self.task_indx += 1;
            task
        }));
    }

    /// Clear the task queue. Ongoing tasks will not be canceled.
    /// Results of ongoing and already completed tasks will remain unaffected.
    fn cancel_tasks(&mut self) {
        self.task_indx -= self.inner.pending_tasks();
        self.inner.cancel_tasks();
    }

    fn get(&mut self) -> Option<R> {
       self.get_in_order(|inner| inner.get())
    }

    fn get_blocking(&mut self) -> Option<R> {
        self.get_in_order(|inner| inner.get_blocking())
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
    fn get_in_order(&mut self, get_function: impl Fn(&mut BasicWorker<(usize, T), (usize, R)>) -> Option<(usize, R)>) -> Option<R> {
        if let Some(&OrderedResult{indx, ..}) = self.result_heap.peek() {
            if indx == self.result_indx {
                self.result_indx += 1;
                let result = self.result_heap.pop().unwrap().result;
                return Some(result);
            }
        }
        while let Some((indx ,result)) = get_function(&mut self.inner) {
            if indx == self.result_indx {
                self.result_indx += 1;
                return Some(result);
            }
            self.result_heap.push(OrderedResult { result, indx });   
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
            result_heap: BinaryHeap::new(),
            task_indx: 0,
            result_indx: 0,
        }
    }
}

