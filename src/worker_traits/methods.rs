/// Methods for interacting with a worker.
pub trait WorkerMethods<T, R> {
    /// Add a task to the end of the queue.
    /// The task will be processed by one of the worker threads.
    fn add_task(&mut self, task: T);

    /// Add multiple tasks to the end of the queue.
    /// The tasks will be processed by the worker threads.
    fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>);

    /// Return the next result. If no result is available, return None.
    /// This function will not block.
    fn get(&mut self) -> Option<R>;

    /// Return the next result. If no result is available block until a result is available.
    /// If no tasks are pending, return None.
    fn get_blocking(&mut self) -> Option<R>;

    /// Cancel all tasks.
    fn cancel_tasks(&mut self);

    /// Return an iterator over all available results.
    /// This function will not block.
    fn get_iter(&mut self) -> impl Iterator<Item = R> {
        std::iter::from_fn(|| self.get())
    }

    /// Returns an iterator over all results.
    /// This function will block until all tasks have been processed.
    fn get_iter_blocking(&mut self) -> impl Iterator<Item = R> {
        std::iter::from_fn(|| self.get_blocking())
    }

    /// Receive all available results and return them in a vector.
    /// This function will not block.
    fn get_vec(&mut self) -> Vec<R> {
        self.get_iter().collect()
    }

    /// Block until all tasks have been processed and return all results in a vector.
    /// This function will block until all tasks have been processed.
    fn get_vec_blocking(&mut self) -> Vec<R> {
        self.get_iter_blocking().collect()
    }

    /// Write available results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all available results, the remaining results will be left in the queue.
    /// This function will not block.
    fn get_buffered(&mut self, buffer: &mut [R]) -> usize {
        write_buffered(buffer, self.get_iter())
    }

    /// Write all results into the buffer and return the number of results written.
    /// If the buffer is too small to hold all results, the remaining results will be left in the queue.
    /// This function will block until all tasks have been processed or the buffer is full.
    fn get_buffered_blocking(&mut self, buffer: &mut [R]) -> usize {
        write_buffered(buffer, self.get_iter_blocking())
    }

    /// Return the number of pending tasks.
    /// This only includes tasks that have been added to the queue but have not started processing.
    fn pending_tasks(&self) -> usize;

    /// Clear the task queue, cancel all tasks and discard all results.
    /// This function will block until all tasks have been canceled.
    fn reset(&mut self) {
        self.cancel_tasks();
        self.get_iter_blocking().for_each(|_| ());
    }
}

fn write_buffered<R>(buffer: &mut [R], it: impl Iterator<Item = R>) -> usize {
    let mut indx = 0;
    for result in it.take(buffer.len()) {
        buffer[indx] = result;
        indx += 1;
    }
    indx
}
