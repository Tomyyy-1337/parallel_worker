use std::{thread::sleep, time::Duration};

use parallel_worker::{State, Worker, check_if_cancelled};

fn main() {
    let worker = Worker::new(worker_function);

    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.cancel_tasks();

    worker.add_task(());
    worker.cancel_tasks();

    worker.get_vec_blocking();
}

fn worker_function(_task: (), state: &State) -> Option<()> {
    for _ in 0.. {
        check_if_cancelled!(state);
        sleep(Duration::from_secs(1));
    }
    return Some(());
}
