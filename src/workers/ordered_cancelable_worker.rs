use super::CancelableWorker;
use crate::{
    State,
    internal::HeapBuffer,
    worker_traits::{WorkerInit, WorkerMethods},
};
use std::num::NonZeroUsize;

/// A worker that processes tasks in parallel using multiple worker threads.
/// The results are returned in same order as the tasks were added.
/// Allows for optional results and task cancelation.
/// 
/// You can use the [`crate::check_if_cancelled!`] macro inside the worker function to check if the task has been canceled.
/// The worker will cancel all tasks as soon as possible when [`Self::cancel_tasks`] is called.
/// The results of canceled tasks will be discarded. Results of already completed tasks will remain unaffected.
pub struct OrderedCancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: CancelableWorker<(NonZeroUsize, T), (NonZeroUsize, R)>,
    result_heap: HeapBuffer<R>,
    task_indx: NonZeroUsize,
}

impl<T, R> WorkerMethods<T, R> for OrderedCancelableWorker<T, R>
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

    /// Clear the task queue and cancel all ongoing tasks as soon as possible.
    /// The results of canceled tasks will be discarded. Results of already completed tasks will remain unaffected.
    /// Canceling tasks during their execution requires the worker function to use the [`crate::check_if_cancelled!`] macro.
    fn cancel_tasks(&mut self) {
        self.task_indx = self.result_heap.current_indx();
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

impl<T, R> OrderedCancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn get_in_order(
        &mut self,
        get_function: impl Fn(
            &mut CancelableWorker<(NonZeroUsize, T), (NonZeroUsize, R)>,
        ) -> Option<(NonZeroUsize, R)>,
    ) -> Option<R> {
        if let Some(result) = self.result_heap.get() {
            return Some(result);
        }
        while let Some((indx, result)) = get_function(&mut self.inner) {
            if let Some(result) = self.result_heap.store_or_return(result, indx) {
                return Some(result);
            }
        }
        None
    }
}

impl<T, R, F> WorkerInit<T, R, F> for OrderedCancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T, &State) -> Option<R> + Send + Copy + 'static,
{
    fn with_num_threads(num_worker_threads: usize, worker_function: F) -> Self {
        let inner =
            CancelableWorker::with_num_threads(num_worker_threads, move |(indx, task), state| {
                worker_function(task, state).map(|result| (indx, result))
            });

        Self {
            inner,
            result_heap: HeapBuffer::new(),
            task_indx: NonZeroUsize::MIN,
        }
    }
}
