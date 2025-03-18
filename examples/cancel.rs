use std::{thread::sleep, time::Duration};

use parallel_worker::{State, Worker, check_if_cancelled};

fn main() {
    let mut worker = Worker::new(worker_function);

    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.add_task(());
    worker.clear_queue();

    worker.add_task(());
    worker.clear_queue();

    worker.get_vec_blocking();
}

fn worker_function(_task: (), state: &State) -> Option<()> {
    for _ in 0.. {
        check_if_cancelled!(state);
        sleep(Duration::from_secs(1));
    }
    return Some(());
}
