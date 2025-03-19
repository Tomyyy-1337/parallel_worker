use std::{thread::sleep, time::Duration};

use parallel_worker::{State, Worker, check_if_cancelled};

fn main() {
    let worker = Worker::with_num_threads(4, worker_function);

    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.cancel_tasks();

    worker.add_task(());
    worker.cancel_tasks();

    
    assert_eq!(worker.get_blocking(), None);
}

fn worker_function(_task: (), state: &State) -> Option<()> {
    for _ in 0.. {
        check_if_cancelled!(state);
        sleep(Duration::from_millis(100));
    }
    return Some(());
}
