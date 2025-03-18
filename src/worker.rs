use std::{
    iter::Peekable, sync::mpsc::{Receiver, Sender}, thread::available_parallelism
};

use crate::{State, task_queue::TaskQueue};

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
    num_pending_tasks: usize,
    worker_state: Vec<State>,
}

impl<T, R> Iterator for Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    type Item = R;

    /// Return the next result. If no result is available, return None.
    /// This function will not block.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.result_receiver.try_recv() {
                Ok(result) => {
                    self.num_pending_tasks -= 1;
                    match result {
                        Some(result) => return Some(result),
                        None => (),
                    }
                }
                Err(_) => return None,
            }
        }
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

        let mut worker_state = Vec::new();
        for _ in 0..num_worker_threads.max(1) {
            let state = State::new();
            worker_state.push(state.clone());
            Self::spawn_worker_thread(
                worker_function,
                result_sender.clone(),
                task_queue.clone(),
                state,
            );
        }

        Worker {
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: 0,
            worker_state,
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
    pub fn clear_queue(&mut self) {
        self.num_pending_tasks -= self.task_queue.clear_queue();
        for state in &self.worker_state {
            state.cancel();
        }
    }

    /// Add a task to the end of the queue.
    pub fn add_task(&mut self, task: T) {
        self.num_pending_tasks += 1;
        self.task_queue.push(Work::Task(task));
    }

    /// Add multiple tasks to the end of the queue.
    pub fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>) {
        let num = self.task_queue.extend(tasks.into_iter().map(Work::Task));
        self.num_pending_tasks += num;
    }

    /// BLock until a result is available and return it.
    fn wait_on_channel(&mut self) -> Option<R> {
        match self.result_receiver.recv() {
            Ok(result) => {
                self.num_pending_tasks -= 1;
                match result {
                    Some(result) => Some(result),
                    None => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Wait for the next result and return it if there are any pending tasks. 
    /// Blocks until a result is available.
    /// If not tasks are pending, this function will return None.
    pub fn wait_for_result(&mut self) -> Option<R> {
        while self.num_pending_tasks > 0 {
            if let Some(result) = self.wait_on_channel() {
                return Some(result);
            }
        }
        None
    }

    /// Block until all tasks have been processed and return all results in a vector.
    pub fn wait_for_all_results(&mut self) -> Vec<R> {
        let mut results = Vec::with_capacity(self.num_pending_tasks);
        while self.num_pending_tasks > 0 {
            match self.wait_on_channel() {
                Some(result) => results.push(result),
                None => (),
            }

        }
        results
    }

    /// Receive all available results and return them in a vector.
    /// This function will not block.
    pub fn receive_all_results(&mut self) -> Vec<R> {
        let mut results = Vec::new();
        loop {
            match self.next() {
                Some(result) => results.push(result),
                None => break,
            }
        }
        results
    }

    /// Write all results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all results, the remaining results will be left in the queue.
    /// This function will block until no tasks are pending or the buffer is full.
    pub fn receive_results_in_buffer_blocking(&mut self, buffer: &mut [R]) -> usize {
        let mut indx = 0;
        while indx < buffer.len() && self.num_pending_tasks > 0 {
            match self.wait_for_result() {
                Some(result) => {
                    buffer[indx] = result;
                    indx += 1;
                }
                None => (),
            }
        }
        indx
    }

    /// Write available results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all available results, the remaining results will be left in the queue.
    /// This function will not block. 
    pub fn receive_results_in_buffer(&mut self, buffer: &mut [R]) -> usize {
        let mut indx = 0;
        while indx < buffer.len() && self.num_pending_tasks > 0 {
            match self.next() {
                Some(result) => {
                    buffer[indx] = result;
                    indx += 1;
                }
                None => break,
            }
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
        self.num_pending_tasks
    }

    fn spawn_worker_thread(
        worker_function: fn(T, &State) -> Option<R>,
        result_sender: Sender<Option<R>>,
        task_queue: TaskQueue<Work<T>>,
        state: State,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                state.set_waiting();
                let task = task_queue.wait_for_task();
                if !state.set_running() {
                    if let Err(_) = result_sender.send(None) {
                        break;
                    }
                }
                match task {
                    Work::Terminate => break,
                    Work::Task(task) => {
                        state.set_running();
                        let result = worker_function(task, &state);
                        let send_value = if state.is_cancelled() { None } else { result };
                            
                        if let Err(_) = result_sender.send(send_value) {
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
        self.clear_queue();

        let messages = (0..self.num_worker_threads).map(|_| Work::Terminate);

        self.task_queue.extend(messages);
    }
}
