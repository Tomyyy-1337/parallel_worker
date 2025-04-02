use super::BasicWorker;
use crate::{
    internal::TaskQueue,
    prelude::State,
    worker_traits::{WorkerInit, WorkerMethods},
};
use std::sync::mpsc::Sender;

/// A worker that processes tasks in parallel using multiple worker threads.
/// Allows for optional results and task cancelation.
/// 
/// You can use the [`crate::check_if_cancelled!`] macro inside the worker function to check if the task has been canceled.
/// The worker will cancel all tasks as soon as possible when [`Self::cancel_tasks`] is called.
/// The results of canceled tasks will be discarded. Results of already completed tasks will remain unaffected.
pub struct CancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: BasicWorker<T, Option<R>>,
    worker_state: Vec<State>,
}

impl<T, R> WorkerMethods<T, R> for CancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&mut self, task: T) {
        self.inner.add_task(task);
    }

    fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>) {
        self.inner.add_tasks(tasks);
    }

    /// Clear the task queue and cancel all ongoing tasks as soon as possible.
    /// The results of canceled tasks will be discarded. Results of already completed tasks will remain unaffected.
    /// Canceling tasks during their execution requires the worker function to use the [`crate::check_if_cancelled!`] macro.
    fn cancel_tasks(&mut self) {
        self.inner.cancel_tasks();
        for state in &self.worker_state {
            state.cancel();
        }
    }

    fn get(&mut self) -> Option<R> {
        self.inner.get_iter().flatten().next()
    }

    fn get_blocking(&mut self) -> Option<R> {
        self.inner.get_iter_blocking().flatten().next()
    }

    fn pending_tasks(&self) -> usize {
        self.inner.pending_tasks()
    }
}

impl<T, R, F> WorkerInit<T, R, F> for CancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T, &State) -> Option<R> + Copy + Send + 'static,
{
    fn with_num_threads(num_worker_threads: usize, worker_function: F) -> Self {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let task_queue = TaskQueue::new();

        let mut worker_state = Vec::with_capacity(num_worker_threads);
        for _ in 0..num_worker_threads {
            let state = State::new();
            spawn_worker_thread(
                worker_function,
                result_sender.clone(),
                task_queue.clone(),
                state.clone(),
            );
            worker_state.push(state);
        }

        CancelableWorker {
            worker_state,
            inner: BasicWorker::constructor(task_queue, result_receiver, num_worker_threads),
        }
    }
}

fn spawn_worker_thread<T, R, F>(
    worker_function: F,
    result_sender: Sender<Option<R>>,
    task_queue: TaskQueue<Option<T>>,
    state: State,
) where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T, &State) -> Option<R> + Send + 'static,
{
    std::thread::spawn(move || {
        loop {
            match task_queue.wait_for_task_and_then(|| state.set_running()) {
                None => break,
                Some(task) => {
                    let result = worker_function(task, &state);
                    let result = if state.is_cancelled() { None } else { result };

                    if let Err(_) = result_sender.send(result) {
                        break;
                    }
                }
            }
        }
    });
}

impl<T, R> Drop for CancelableWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Drop the worker and terminate all worker threads. Cancel all tasks as soon as possible.
    fn drop(&mut self) {
        for state in &self.worker_state {
            state.cancel();
        }
    }
}
