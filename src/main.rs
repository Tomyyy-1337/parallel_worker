use std::{thread::sleep, time::Duration};

use parallel_worker::{BasicWorker, OrderedWorker, WorkerInit, WorkerMethods};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

const NUMBER_OF_TASKS: u64 = 70_000;

fn main() { 
    let mut worker = BasicWorker::with_num_threads(28,worker_function);    
    
    let start = std::time::Instant::now();

    worker.add_tasks(1..=NUMBER_OF_TASKS);

    let count = worker.get_iter_blocking().sum::<u64>();
    
    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);
    println!("Number of primes: {}", count);

    // Baseline
    let baseline_start = std::time::Instant::now();
    let result = (1..=NUMBER_OF_TASKS)
        .into_par_iter()
        .map(worker_function)
        .sum::<u64>();

    let baseline_duration = baseline_start.elapsed();
    println!("Baseline time taken: {:?}", baseline_duration);
    println!("Baseline number of primes: {}", result);
}

fn worker_function(n: u64) -> u64 {
    // prime sive of Eratosthenes
    let mut sieve = vec![true; (n + 1) as usize];
    sieve[0] = false; // 0 is not prime
    sieve[1] = false; // 1 is not prime

    for i in 2..=((n as f64).sqrt() as u64) {
        if sieve[i as usize] {
            for j in (i * i..=n).step_by(i as usize) {
                sieve[j as usize] = false;
            }
        }
    }

    // number of primes
    sieve.iter().filter(|&&is_prime| is_prime).count() as u64
}