use std::{
    cell::Cell,
    sync::mpsc::{Receiver, Sender},
    thread::available_parallelism,
};

use crate::{cell_utils::CellUpdate, task_queue::TaskQueue, WorkerMethods};

enum Work<T> {
    Task(T),
    Terminate,
}

/// A worker that processes tasks in parallel using multiple worker threads. 
pub struct BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    task_queue: TaskQueue<Work<T>>,
    result_receiver: Receiver<R>,
    num_worker_threads: usize,
    num_pending_tasks: Cell<usize>,
}

impl<T, R> WorkerMethods<T, R> for BasicWorker<T, R> 
where 
    T: Send + 'static,
    R: Send + 'static,
{
    /// Add a task to the end of the queue. 
    /// The task will be processed by one of the worker threads.
    fn add_task(&self, task: T) {
        self.num_pending_tasks.modify(|n| n + 1);
        self.task_queue.push(Work::Task(task));
    }
    
    /// Add multiple tasks to the end of the queue.
    /// The tasks will be processed by the worker threads.
    fn add_tasks(&self, tasks: impl IntoIterator<Item = T>){
        let num = self.task_queue.extend(tasks.into_iter().map(Work::Task));
        self.num_pending_tasks.modify(|n| n + num);
    }
    
     /// Clear the task queue. Ongoing tasks will not be canceled. 
    fn cancel_tasks(&self) {
        let tasks_in_queue = self.task_queue.clear_queue();
        self.num_pending_tasks.modify(|n| n - tasks_in_queue);
    }

    /// Return the next result. If no result is available, return None.
    /// This function will not block.
    fn get(&self) -> Option<R> {
        self.num_pending_tasks.modify(|n| n - 1);
        if let Ok(result) = self.result_receiver.try_recv() {
            return Some(result);
        }
        None
    }

    /// Return the next result. If no result is available block until a result is available.
    /// If no tasks are pending, return None.
    fn get_blocking(&self) -> Option<R> {
        if self.num_pending_tasks.get() > 0 {
            self.num_pending_tasks.modify(|n| n - 1);
            return self.result_receiver.recv().ok();
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

impl<T, R> BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Create a new worker with a given number of worker threads and a worker function.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn with_num_threads(
        num_worker_threads: usize,
        worker_function: fn(T) -> R,
    ) -> BasicWorker<T, R> {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let task_queue = TaskQueue::new();

        Self::start_worker_threads(
            num_worker_threads,
            worker_function,
            result_sender,
            &task_queue,
        );

        BasicWorker {
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: Cell::new(0),
        }
    }

    /// Create a new worker with a given worker function. The number of worker threads will be set to the number of available
    /// logical cores minus one. If you want to use a custom thread count, use the `with_num_threads` method to create a worker.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    pub fn new(worker_function: fn(T) -> R) -> BasicWorker<T, R> {
        let num_worker_threads = available_parallelism()
            .map(|n| n.get().saturating_sub(1))
            .unwrap_or(1);
        Self::with_num_threads(num_worker_threads, worker_function)
    }

    fn start_worker_threads(
        num_worker_threads: usize,
        worker_function: fn(T) -> R,
        result_sender: Sender<R>,
        task_queue: &TaskQueue<Work<T>>,
    ) {
        for _ in 0..num_worker_threads {
            Self::spawn_worker_thread(
                worker_function,
                result_sender.clone(),
                task_queue.clone(),
            );
        }
    }

    fn spawn_worker_thread(
        worker_function: fn(T) -> R,
        result_sender: Sender<R>,
        task_queue: TaskQueue<Work<T>>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                match task_queue.wait_for_task_and_then(|| ()) {
                    Work::Terminate => break,
                    Work::Task(task) => {
                        let result = worker_function(task);

                        if let Err(_) = result_sender.send(result) {
                            break;
                        }
                    }
                }
            }
        })
    }
}

impl<T, R> Drop for BasicWorker<T, R>
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
