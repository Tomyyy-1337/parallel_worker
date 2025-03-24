use std::{cell::Cell, sync::mpsc::Receiver};

use crate::{
    cell_utils::CellUpdate,
    task_queue::TaskQueue,
    worker_traits::{Work, WorkerInit, WorkerMethods},
};

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
    fn add_task(&self, task: T) {
        self.task_queue.push(Work::Task(task));
        self.num_pending_tasks.modify(|n| n + 1);
    }

    fn add_tasks(&self, tasks: impl IntoIterator<Item = T>) {
        let num = self.task_queue.extend(tasks.into_iter().map(Work::Task));
        self.num_pending_tasks.modify(|n| n + num);
    }

    /// Clear the task queue. Ongoing tasks will not be canceled.
    fn cancel_tasks(&self) {
        let tasks_in_queue = self.task_queue.clear_queue();
        self.num_pending_tasks.modify(|n| n - tasks_in_queue);
    }

    fn get(&self) -> Option<R> {
        if let Ok(result) = self.result_receiver.try_recv() {
            self.num_pending_tasks.modify(|n| n - 1);
            return Some(result);
        }
        None
    }

    fn get_blocking(&self) -> Option<R> {
        if self.num_pending_tasks.get() > 0 {
            self.num_pending_tasks.modify(|n| n - 1);
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
    task_queue: TaskQueue<Work<T>>,
) where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Copy + 'static,
{
    std::thread::spawn(move || {
        loop {
            match task_queue.wait_for_task_and_then(|| ()) {
                Work::Terminate => break,
                Work::Task(task) => {
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
        task_queue: TaskQueue<Work<T>>,
        result_receiver: Receiver<R>,
        num_worker_threads: usize,
    ) -> Self {
        Self {
            task_queue,
            result_receiver,
            num_worker_threads,
            num_pending_tasks: Cell::new(0),
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

        let messages = (0..self.num_worker_threads).map(|_| Work::Terminate);

        self.task_queue.extend(messages);
    }
}
