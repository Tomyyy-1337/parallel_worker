use std::{thread::sleep, time::Duration};

use parallel_worker::{check_if_cancelled, State, Worker};

fn main() {
    let mut worker = Worker::new(
        4,
        worker_function,
    );

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let results = worker.wait_for_all_results();
    println!("Results: {:?}", results);
}

fn worker_function(task: u64, state: &State) -> Option<u64> {
    check_if_cancelled!(state);
    
    sleep(Duration::from_secs(task));
    
    check_if_cancelled!(state);

    println!("Task completed after {} seconds", task);

    Some(task)
}
