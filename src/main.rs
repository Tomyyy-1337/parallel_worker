use parallel_worker::prelude::*;

fn main() {
    let worker = OrderedWorker::new(|n: u64| {
        fib(n % 49)
    });

    let start = std::time::Instant::now();
    worker.add_task(0);
    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);
    worker.add_task(4);
    worker.add_task(5);
    worker.add_tasks(6..=10);
    worker.add_tasks(11..=20);
    worker.add_tasks(21..=1000);
    worker.add_tasks(1001..=100000);

    let mut s = 0;
    for result in worker.get_iter_blocking() {
        println!("Result: {}", result);
        s += result;
    }
    println!("Time: {:?}", start.elapsed());
    println!("Sum: {}", s);
}

fn fib(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}