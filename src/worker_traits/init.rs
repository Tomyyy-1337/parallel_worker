use std::thread::available_parallelism;

/// Methods for creating a worker.
pub trait WorkerInit<T, R, F> 
where 
    Self: Sized,
{
    /// Create a new worker with a given number of worker threads and a worker function.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    fn with_num_threads(num_worker_threads: usize, worker_function: F) -> Self;

    /// Create a new worker with a given worker function. 
    /// The number of worker threads will be set to the number of available logical cores minus one.
    /// Spawns worker threads that will process tasks from the queue using the worker function.
    fn new(worker_function: F) -> Self {
        let num_worker_threads = available_parallelism()
            .map(|n| n.get().saturating_sub(1))
            .unwrap_or(1);
        Self::with_num_threads(num_worker_threads, worker_function)
    }
}

