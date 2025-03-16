use std::sync::mpsc::{Receiver, Sender};

use crate::{task_queue::TaskQueue, State};

enum Work<T> {
    Task(T),
    Terminate,
}

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

impl<T, R> Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Create a new worker with a given number of worker threads and a worker function.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn new(num_worker_threads: usize, worker_function: fn(T, &State) -> Option<R>) -> Worker<T, R> {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let task_queue = TaskQueue::new();
        
        let mut worker_state = Vec::new();
        for _ in 0..num_worker_threads.max(1) {
            let state = State::new();
            worker_state.push(state.clone());
            Self::spawn_worker_thread(worker_function, result_sender.clone(), task_queue.clone(), state);
        }

        Worker {
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: 0,
            worker_state,
        }
    }

    /// Clear the task queue and cancel all tasks as soon as possible.
    pub fn clear_queue(&mut self) {
        self.task_queue.clear_queue();
        for state in &self.worker_state {
            state.cancel();
        }
        self.num_pending_tasks = 0;
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

    /// Return the next result. If no result is available, return None.
    /// This function will not block.
    pub fn get_result_option(&mut self) -> Option<R> {
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

    /// Wait for the next result and return it. Blocks until a result is available.
    pub fn wait_for_result(&mut self) -> R {
        loop {
            self.num_pending_tasks = self.num_pending_tasks.saturating_sub(1);
            match self.result_receiver.recv().unwrap() {
                Some(result) => {
                    return result;
                },
                None => (),
            }
        }
    }

    /// Block until all tasks have been processed and return all results in a vector.
    pub fn wait_for_all_results(&mut self) -> Vec<R> {
        let mut results = Vec::with_capacity(self.num_pending_tasks);
        while self.num_pending_tasks > 0 {
            let result = self.wait_for_result();
            results.push(result);
        }
        results
    }

    /// Receive all available results and return them in a vector.
    /// This function will not block.
    pub fn receive_all_results(&mut self) -> Vec<R> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_receiver.try_recv() {
            match result {
                Some(result) => results.push(result),
                None => (),
            }
        }
        self.num_pending_tasks -= results.len();
        results
    }

    /// Write available results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all available results, the remaining results will be left in the queue.
    /// This function will not block.
    pub fn receive_results_in_buffer(&mut self, buffer: &mut [R]) -> usize {
        let mut indx = 0;
        while indx < buffer.len() {
            match self.result_receiver.try_recv() {
                Ok(Some(result)) => {
                    buffer[indx] = result;
                    indx += 1;
                }
                Ok(None) => (),
                Err(_) => break,
            }
        }
        self.num_pending_tasks -= indx;
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
                match task_queue.wait_for_task() {
                    Work::Terminate => break,
                    Work::Task(task) => {
                        state.set_running();
                        let result =  worker_function(task, &state);
                        if !state.is_cancelled() {
                            if let Err(_) = result_sender.send(result) {
                                break;
                            }
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
