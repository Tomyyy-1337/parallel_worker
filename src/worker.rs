use std::{
    sync::mpsc::Sender,
    thread::available_parallelism,
};

use crate::{task_queue::TaskQueue, worker_methods::{Work, WorkerMethods}, BasicWorker, State};

/// A worker that processes tasks in parallel using multiple worker threads. 
/// Allows for optional results and task cancelation.
pub struct Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    inner: BasicWorker<T, Option<R>>,
    worker_state: Vec<State>,
}

impl<T, R> WorkerMethods<T, R> for Worker<T, R> 
where 
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&self, task: T) {
        self.inner.add_task(task);
    }

    fn add_tasks(&self, tasks: impl IntoIterator<Item = T>) {
        self.inner.add_tasks(tasks);
    }

    /// Clear the task queue and cancel all tasks as soon as possible. 
    /// The results of canceled tasks will be discarded.
    /// Canceling the execution of tasks requires the worker function to use the `check_if_cancelled!` macro.
    fn cancel_tasks(&self) {
        self.inner.cancel_tasks();
        for state in &self.worker_state {
            state.cancel();
        }
    }

    fn get(&self) -> Option<R> {
        self.inner.get_iter().flatten().next()
    }

    fn get_blocking(&self) -> Option<R> {
        self.inner.get_iter_blocking().flatten().next()
    }

    fn current_queue_size(&self) -> usize {
        self.inner.current_queue_size()
    }

    fn num_pending_tasks(&self) -> usize {
        self.inner.num_pending_tasks()
    }
}

impl<T, R> Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Create a new worker with a given number of worker threads and a worker function.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn with_num_threads(
        num_worker_threads: usize,
        worker_function: fn(T, &State) -> Option<R>,
    ) -> Worker<T, R> {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let task_queue = TaskQueue::new();

        Worker {
            worker_state: Self::start_worker_threads(
                num_worker_threads,
                worker_function,
                result_sender,
                &task_queue,
            ),
            inner: BasicWorker::constructor(task_queue, result_receiver, num_worker_threads)
        }
    }

    /// Create a new worker with a given worker function. The number of worker threads will be set to the number of available
    /// logical cores minus one. If you want to use a custom thread count, use the `with_num_threads` method to create a worker.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn new(worker_function: fn(T, &State) -> Option<R>) -> Worker<T, R> {
        let num_worker_threads = available_parallelism()
            .map(|n| n.get().saturating_sub(1))
            .unwrap_or(1);
        Self::with_num_threads(num_worker_threads, worker_function)
    }

    fn start_worker_threads(
        num_worker_threads: usize,
        worker_function: fn(T, &State) -> Option<R>,
        result_sender: Sender<Option<R>>,
        task_queue: &TaskQueue<Work<T>>,
    ) -> Vec<State> {
        let mut worker_states = Vec::with_capacity(num_worker_threads);
        for _ in 0..num_worker_threads {
            let state = State::new();
            Self::spawn_worker_thread(
                worker_function,
                result_sender.clone(),
                task_queue.clone(),
                state.clone(),
            );
            worker_states.push(state);
        }
        worker_states
    }

    fn spawn_worker_thread(
        worker_function: fn(T, &State) -> Option<R>,
        result_sender: Sender<Option<R>>,
        task_queue: TaskQueue<Work<T>>,
        state: State,
    ) {
        std::thread::spawn(move || {
            loop {
                match task_queue.wait_for_task_and_then(|| state.set_running()) {
                    Work::Terminate => break,
                    Work::Task(task) => {
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
}

impl<T, R> Drop for Worker<T, R>
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
