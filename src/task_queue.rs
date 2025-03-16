use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

struct InnerTaskQueue<T> {
    tasks: Mutex<VecDeque<T>>,
    condvar: Condvar,
}

impl<T> InnerTaskQueue<T> {
    fn len(&self) -> usize {
        self.tasks.lock().unwrap().len()
    }

    fn clear_queue(&self) -> usize {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.drain(..).len()
    }

    fn new() -> InnerTaskQueue<T> {
        InnerTaskQueue {
            tasks: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        }
    }

    fn push(&self, task: T) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
        self.condvar.notify_one();
    }

    fn extend(&self, new_tasks: impl IntoIterator<Item = T>) -> usize {
        let num_new = {
            let mut tasks = self.tasks.lock().unwrap();
            let len = tasks.len();
            tasks.extend(new_tasks);
            tasks.len() - len
        };
        match num_new {
            0 => (),
            1 => self.condvar.notify_one(),
            _ => self.condvar.notify_all(),
        }
        num_new
    }

    fn wait_for_task(&self) -> T {
        let mut tasks = self.tasks.lock().unwrap();
        loop {
            match tasks.pop_front() {
                Some(task) => return task,
                None => tasks = self.condvar.wait(tasks).unwrap(),
            }
        }
    }
}

pub struct TaskQueue<T> {
    inner: Arc<InnerTaskQueue<T>>,
}

impl<T> TaskQueue<T> {
    pub fn new() -> TaskQueue<T> {
        TaskQueue {
            inner: Arc::new(InnerTaskQueue::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn clear_queue(&self) -> usize {
        self.inner.clear_queue()
    }

    pub fn push(&self, task: T) {
        self.inner.push(task);
    }

    pub fn extend(&self, new_tasks: impl IntoIterator<Item = T>) -> usize {
        self.inner.extend(new_tasks)
    }

    pub fn wait_for_task(&self) -> T {
        self.inner.wait_for_task()
    }
}

impl<T> Clone for TaskQueue<T> {
    fn clone(&self) -> TaskQueue<T> {
        TaskQueue {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn test_task_queue() {
        let task_queue = TaskQueue::new();
        assert_eq!(task_queue.len(), 0);
        task_queue.push(1);
        assert_eq!(task_queue.len(), 1);
        task_queue.push(2);
        assert_eq!(task_queue.len(), 2);
        assert_eq!(task_queue.wait_for_task(), 1);
        assert_eq!(task_queue.len(), 1);
        assert_eq!(task_queue.wait_for_task(), 2);
        assert_eq!(task_queue.len(), 0);

        let task_queue_clone = task_queue.clone();
        let t = std::thread::spawn(move || {
            task_queue_clone.wait_for_task();
        });

        sleep(Duration::from_millis(500));
        assert!(!t.is_finished());

        task_queue.push(1);
        assert!(t.join().is_ok());

        let threads = (0..10)
            .map(|_| {
                let task_queue_clone = task_queue.clone();
                std::thread::spawn(move || {
                    task_queue_clone.wait_for_task();
                })
            })
            .collect::<Vec<_>>();

        assert!(threads.iter().all(|t| !t.is_finished()));
        task_queue.extend(0..1);
        assert!(threads.iter().any(|t| !t.is_finished()));
        task_queue.extend(0..0);
        assert!(threads.iter().any(|t| !t.is_finished()));
        task_queue.extend(0..8);
        assert!(threads.iter().any(|t| !t.is_finished()));
        task_queue.extend(0..1);

        assert!(threads.into_iter().all(|t| t.join().is_ok()));
    }
}
