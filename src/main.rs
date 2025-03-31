use parallel_worker::prelude::*;

fn main() {
    let worker = OrderedWorker::new(|n: u64| {
        std::thread::sleep(std::time::Duration::from_secs(n % 5));
        n
    });

    worker.add_task(0);
    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);
    worker.add_task(4);
    worker.add_task(5);
    worker.add_tasks(6..=10);
    worker.add_tasks(11..=20);
    worker.add_tasks(21..=1000);

    for result in worker.get_iter_blocking() {
        println!("Result: {}", result);
    }
}