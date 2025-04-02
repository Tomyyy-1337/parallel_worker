use std::sync::mpsc::Receiver;

use crate::{
    internal::TaskQueue, worker_traits::{WorkerInit, WorkerMethods}
};

/// A worker that processes tasks in parallel using multiple worker threads.
pub struct BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    task_queue: TaskQueue<Option<T>>,
    result_receiver: Receiver<R>,
    num_worker_threads: usize,
    num_pending_tasks: usize,
}

impl<T, R> WorkerMethods<T, R> for BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn add_task(&mut self, task: T) {
        self.task_queue.push(Some(task));
        self.num_pending_tasks += 1;
    }

    fn add_tasks(&mut self, tasks: impl IntoIterator<Item = T>) {
        let num = self.task_queue.extend(tasks.into_iter().map(Some));
        self.num_pending_tasks += num;
    }

    /// Clear the task queue. Ongoing tasks will not be canceled.
    /// Results of ongoing and already completed tasks will remain unaffected.
    fn cancel_tasks(&mut self) {
        let tasks_in_queue = self.task_queue.clear_queue();
        self.num_pending_tasks -= tasks_in_queue;
    }

    fn get(&mut self) -> Option<R> {
        if let Ok(result) = self.result_receiver.try_recv() {
            self.num_pending_tasks -= 1;
            return Some(result);
        }
        None
    }

    fn get_blocking(&mut self) -> Option<R> {
        if self.num_pending_tasks > 0 {
            self.num_pending_tasks -= 1;
            return self.result_receiver.recv().ok();
        }
        None
    }

    fn pending_tasks(&self) -> usize {
        self.task_queue.len()
    }
}

impl<T, R, F> WorkerInit<T, R, F> for BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Copy + 'static,
{
    fn with_num_threads(num_worker_threads: usize, worker_function: F) -> Self {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let task_queue = TaskQueue::new();

        for _ in 0..num_worker_threads {
            spawn_worker_thread(
                worker_function,
                result_sender.clone(),
                task_queue.clone(),
            );
        }

        BasicWorker::constructor(task_queue, result_receiver, num_worker_threads)
    }
}

fn spawn_worker_thread<T, R, F>(
    worker_function: F,
    result_sender: std::sync::mpsc::Sender<R>,
    task_queue: TaskQueue<Option<T>>,
) where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Copy + 'static,
{
    std::thread::spawn(move || {
        loop {
            match task_queue.wait_for_task_and_then(|| ()) {
                None => break,
                Some(task) => {
                    if let Err(_) = result_sender.send(worker_function(task)) {
                        break;
                    }
                }
            }
        }
    });
}

impl<T, R> BasicWorker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    pub(crate) fn constructor(
        task_queue: TaskQueue<Option<T>>,
        result_receiver: Receiver<R>,
        num_worker_threads: usize,
    ) -> Self {
        Self {
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: 0,
        }
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

        let messages = (0..self.num_worker_threads).map(|_| None);

        self.task_queue.extend(messages);
    }
}