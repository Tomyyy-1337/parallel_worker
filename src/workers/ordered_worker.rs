use super::BasicWorker;
use crate::{
    internal::HeapBuffer,
    worker_traits::{WorkerInit, WorkerMethods},
};
use std::num::NonZeroUsize;

/// A worker that processes tasks in parallel using multiple worker threads.
/// The results are returned in same order as the tasks were added.
pub struct OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: BasicWorker<(NonZeroUsize, T), (NonZeroUsize, R)>,
    result_heap: HeapBuffer<R>,
    task_indx: NonZeroUsize,
}

impl<T, R> WorkerMethods<T, R> for OrderedWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&mut self, task: T) {
        self.inner.add_task((self.task_indx, task));
        self.task_indx = self.task_indx.saturating_add(1);
    }

    fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>) {
        self.inner.add_tasks(tasks.into_iter().map(|t| {
            let task = (self.task_indx, t);
            self.task_indx = self.task_indx.saturating_add(1);
            task
        }));
    }

    /// Clear the task queue. Ongoing tasks will not be canceled.
    /// Results of ongoing and already completed tasks will remain unaffected.
    fn cancel_tasks(&mut self) {
        let new_indx = self.task_indx.get() - self.inner.pending_tasks();
        self.task_indx = NonZeroUsize::new(new_indx).unwrap();
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
    fn get_in_order(
        &mut self,
        get_function: impl Fn(
            &mut BasicWorker<(NonZeroUsize, T), (NonZeroUsize, R)>,
        ) -> Option<(NonZeroUsize, R)>,
    ) -> Option<R> {
        if let Some(result) = self.result_heap.get() {
            return Some(result);
        }
        while let Some((indx, result)) = get_function(&mut self.inner) {
            if indx == self.result_heap.current_indx() {
                self.result_heap.skip();
                return Some(result);
            }
            self.result_heap.push(result, indx);
        }
        None
    }
}

impl<T, R, F> WorkerInit<T, R, F> for OrderedWorker<T, R>
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
            result_heap: HeapBuffer::new(),
            task_indx: NonZeroUsize::MIN,
        }
    }
}
