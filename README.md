# Parallel Worker
This crate provides a simple interface for running tasks in parallel.
The `Worker` struct is used to dispatch tasks to worker threads and collect the results.
You can wait for results or reviece currently available results.
```rust
 use parallel_worker::{State, Worker};
 fn main() {
    let worker = Worker::new(|n: u64, _s: &State| Some(42));
    worker.add_task(1);
    worker.add_task(2);
    assert_eq!(worker.get_blocking(), Some(42));
    
    worker.add_tasks(0..10);
    assert_eq!(worker.get_iter_blocking().count(), 11);
}
```
## Tasks can be canceled
Canceled tasks will stop executing as soon as they reach a `check_if_cancelled!`.
Results of canceled tasks will be discarded even if they have already been computed.   
```rust
fn main() {
    let worker = Worker::new(worker_function);
    worker.add_task(1);
    worker.cancel_tasks();
    assert!(worker.get_blocking().is_none());
}
fn worker_function(task: u64, state: &State) -> Option<u64> {
    for i in 0.. {
        sleep(Duration::from_millis(50)); // Do some work
        check_if_cancelled!(state); // Check if the task has been canceled
    }
    Some(42)
}
```
## Results can be optinal
If a worker returns `None` the result will be discarded. 
```rust
fn main() {
    let worker = Worker::new(|n: u64, _s: &State| {
        if n % 2 == 0 {
            Some(n)
        } else {
            None
        }
    });
    
    worker.add_tasks(1..=10);

    assert_eq!(worker.get_iter_blocking().count(), 5); 
}
```
