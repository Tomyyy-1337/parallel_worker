use std::{
    cell::Cell,
    sync::mpsc::{Receiver, Sender},
    thread::available_parallelism,
};

use crate::{State, cell_utils::CellUpdate, task_queue::TaskQueue};

enum Work<T> {
    Task(T),
    Terminate,
}

/// A worker that processes tasks in parallel using multiple worker threads.
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
    /// logical cores minus one by default. If you want to use a custom thread count, use the `with_num_threads` method to create a worker.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn new(worker_function: fn(T, &State) -> Option<R>) -> Worker<T, R> {
        let num_worker_threads = available_parallelism()
            .map(|n| n.get().saturating_sub(1))
            .unwrap_or(1);
        Self::with_num_threads(num_worker_threads, worker_function)
    }

    /// Clear the task queue and cancel all tasks as soon as possible.
    pub fn cancel_tasks(&self) {
        let tasks_in_queue = self.task_queue.clear_queue();
        self.num_pending_tasks.modify(|n| n - tasks_in_queue);
        for state in &self.worker_state {
            state.cancel();
        }
    }

    /// Add a task to the end of the queue.
    pub fn add_task(&self, task: T) {
        self.num_pending_tasks.modify(|n| n + 1);
        self.task_queue.push(Work::Task(task));
    }

    /// Add multiple tasks to the end of the queue.
    pub fn add_tasks(&self, tasks: impl IntoIterator<Item = T>) {
        let num = self.task_queue.extend(tasks.into_iter().map(Work::Task));
        self.num_pending_tasks.modify(|n| n + num);
    }

    /// Return the next result. If no result is available, return None.
    /// This function will not block.
    pub fn get(&self) -> Option<R> {
        while let Ok(result) = self.result_receiver.try_recv() {
            self.num_pending_tasks.modify(|n| n - 1);
            if let Some(result) = result {
                return Some(result);
            }
        }
        None
    }

    /// Return the next result. If no result is available block until a result is available.
    /// If no tasks are pending, return None.
    pub fn get_blocking(&self) -> Option<R> {
        while self.num_pending_tasks.get() > 0 {
            self.num_pending_tasks.modify(|n| n - 1);
            if let Ok(Some(result)) = self.result_receiver.recv() {
                return Some(result);
            }
        }
        None
    }

    /// Return an iterator over all available results.
    /// This function will not block.
    pub fn get_iter(&self) -> impl Iterator<Item = R> {
        std::iter::from_fn(|| self.get())
    }

    /// Returns an iterator over all results.
    /// This function will block until all tasks have been processed.
    pub fn get_iter_blocking(&self) -> impl Iterator<Item = R> {
        std::iter::from_fn(|| self.get_blocking())
    }

    /// Receive all available results and return them in a vector.
    /// This function will not block.
    pub fn get_vec(&self) -> Vec<R> {
        self.get_iter().collect()
    }

    /// Block until all tasks have been processed and return all results in a vector.
    pub fn get_vec_blocking(&self) -> Vec<R> {
        self.get_iter_blocking().collect()
    }

    /// Write available results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all available results, the remaining results will be left in the queue.
    /// This function will not block.
    pub fn get_buffered(&self, buffer: &mut [R]) -> usize {
        self.write_buffered(buffer, self.get_iter())
    }

    /// Write all results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all results, the remaining results will be left in the queue.
    /// This function will block until no tasks are pending or the buffer is full.
    pub fn get_buffered_blocking(&self, buffer: &mut [R]) -> usize {
        self.write_buffered(buffer, self.get_iter_blocking())
    }

    fn write_buffered(&self, buffer: &mut [R], it: impl Iterator<Item = R>) -> usize {
        let mut indx = 0;
        for result in it.take(buffer.len()) {
            buffer[indx] = result;
            indx += 1;
        }
        indx
    }

    /// Return the number of tasks currently in the queue. This does not include tasks that are currently being processed
    /// by worker threads.
    pub fn current_queue_size(&self) -> usize {
        self.task_queue.len()
    }

    /// Return the number of pebding tasks. This includes tasks that are currently being processed
    /// by worker threads and tasks that are in the queue.
    pub fn num_pending_tasks(&self) -> usize {
        self.num_pending_tasks.get()
    }

    fn start_worker_threads(
        num_worker_threads: usize,
        worker_function: fn(T, &State) -> Option<R>,
        result_sender: Sender<Option<R>>,
        task_queue: &TaskQueue<Work<T>>,
    ) -> Vec<State> {
        (0..num_worker_threads.max(1))
            .map(|_| State::new())
            .inspect(|state| {
                Self::spawn_worker_thread(
                    worker_function,
                    result_sender.clone(),
                    task_queue.clone(),
                    state.clone(),
                );
            })
            .collect()
    }

    fn spawn_worker_thread(
        worker_function: fn(T, &State) -> Option<R>,
        result_sender: Sender<Option<R>>,
        task_queue: TaskQueue<Work<T>>,
        state: State,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                let task = task_queue.wait_for_task_and_then(|| state.set_running());
                match task {
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
        })
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
