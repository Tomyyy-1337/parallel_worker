use std::{thread::sleep, time::Duration};

use parallel_worker::{State, Worker, check_if_cancelled};

fn main() {
    let worker = Worker::new(worker_function);

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let results = worker.get_vec_blocking();
    println!("Results: {:?}", results);
}

fn worker_function(task: u64, state: &State) -> Option<u64> {
    check_if_cancelled!(state);

    sleep(Duration::from_secs(task));

    check_if_cancelled!(state);

    println!("Task completed after {} seconds", task);

    Some(task)
}
