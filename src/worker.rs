use std::{
    cell::Cell,
    sync::mpsc::{Receiver, Sender},
    thread::available_parallelism,
};

use crate::{cell_utils::CellUpdate, task_queue::TaskQueue, worker_methods::{Work, WorkerMethods}, State};

/// A worker that processes tasks in parallel using multiple worker threads. 
/// Allows for optional results and task cancelation.
pub struct Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    task_queue: TaskQueue<Work<T>>,
    result_receiver: Receiver<Option<R>>,
    num_worker_threads: usize,
    num_pending_tasks: Cell<usize>,
    worker_state: Vec<State>,
}

impl<T, R> WorkerMethods<T, R> for Worker<T, R> 
where 
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&self, task: T) {
        self.num_pending_tasks.modify(|n| n + 1);
        self.task_queue.push(Work::Task(task));
    }
    
    fn add_tasks(&self, tasks: impl IntoIterator<Item = T>) {
        let num = self.task_queue.extend(tasks.into_iter().map(Work::Task));
        self.num_pending_tasks.modify(|n| n + num);
    }

    /// Clear the task queue and cancel all tasks as soon as possible. 
    /// The results of canceled tasks will be discarded.
    /// Canceling the execution of tasks requires the worker function to use the `check_if_cancelled!` macro.
    fn cancel_tasks(&self) {
        let tasks_in_queue = self.task_queue.clear_queue();
        self.num_pending_tasks.modify(|n| n - tasks_in_queue);
        for state in &self.worker_state {
            state.cancel();
        }
    }

    fn get(&self) -> Option<R> {
        while let Ok(result) = self.result_receiver.try_recv() {
            self.num_pending_tasks.modify(|n| n - 1);
            if let Some(result) = result {
                return Some(result);
            }
        }
        None
    }

    fn get_blocking(&self) -> Option<R> {
        while self.num_pending_tasks.get() > 0 {
            self.num_pending_tasks.modify(|n| n - 1);
            if let Ok(Some(result)) = self.result_receiver.recv() {
                return Some(result);
            }
        }
        None
    }

    fn current_queue_size(&self) -> usize {
        self.task_queue.len()
    }

    fn num_pending_tasks(&self) -> usize {
        self.num_pending_tasks.get()
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
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: Cell::new(0),
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
    /// Drop the worker and terminate all worker threads.
    fn drop(&mut self) {
        self.cancel_tasks();

        let messages = (0..self.num_worker_threads).map(|_| Work::Terminate);

        self.task_queue.extend(messages);
    }
}
