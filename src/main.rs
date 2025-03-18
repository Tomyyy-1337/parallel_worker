use std::{thread::sleep, time::Duration};

use parallel_worker::{State, Worker};

fn main() {
    let mut worker = Worker::new(|n: u64, _s: &State| {
        sleep(Duration::from_secs(n));
        Some(n)
    });
    
    worker.add_task(7);
    worker.add_task(1);
    worker.add_task(5);
    worker.add_task(2);
    worker.add_task(6);
    worker.add_task(3);
    worker.add_task(4);
    worker.add_task(8);

    for result in worker.get_iter_blocking() {
        println!("Result: {:?}", result);
    }
}