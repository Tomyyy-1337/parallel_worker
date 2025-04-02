# Parallel Worker

This [crate](https://crates.io/crates/parallel_worker) provides a simple interface for running tasks in parallel.
The Worker structs are used to dispatch tasks to worker threads and collect the results. You can wait for results or recieve currently available results.

## Workers
There are four types of workers:
- `BasicWorker` is a simple worker that processes tasks in parallel using multiple worker threads.
- `CancelableWorker` has additional functionality for optional results and task cancelation during execution.
- `OrderedWorker` returns results in the same order as the tasks were added. 
- `OrderedCancelableWorker` combines the features of `OrderedWorker` and `CancelableWorker`.

## Example
Basic example of using a worker to run tasks in parallel using the `BasicWorker` struct.
Tasks start executing as soon as they are added. When all threads are busy, tasks are queued until a thread becomes available.
```rust
 use parallel_worker::prelude::*;

 fn main() {
    let mut worker = BasicWorker::new(|n| {
       // Answer to life, the universe and everything
       return 42;
    });
    worker.add_task(1);
    worker.add_task(2);
    assert_eq!(worker.get_blocking(), Some(42));
    
    worker.add_tasks(0..10);
    assert_eq!(worker.get_iter_blocking().count(), 11);
}
```
## Tasks can be canceled
If you want to cancel tasks during execution, use `CancelableWorker` or `OrderedCancelableWorker` and call the `check_if_cancelled!` macro in your worker function on a regular basis. 
Exessive checking will lead to a performance costs. Canceled tasks will stop executing as soon as they reach a `check_if_cancelled!`.
Results of canceled tasks will be discarded.
Results of tasks that have already completed will remain unaffected.  
```rust
use parallel_worker::prelude::*;

fn main() {
    let mut worker = CancelableWorker::new(worker_function);
    worker.add_tasks([1, 2, 3, 4]);
    worker.cancel_tasks();
    assert!(worker.get_blocking().is_none());
}

fn worker_function(task: u64, state: &State) -> Option<u64> {
    loop {
        sleep(Duration::from_millis(50)); 
        check_if_cancelled!(state); 
    }
    unreachable!() 
}
```

## Results can be optional
If a worker returns `None` the result will be discarded. This feature is available in the `CancelableWorker` and `OrderedCancelableWorker`.
 
```rust
use parallel_worker::prelude::*;

fn main() {
    let mut worker = CancelableWorker::new(|n: u64, _s: &State| {
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

## Results can be ordered
If you want to get results in the same order as the tasks were added, use `OrderedWorker` or `OrderedCancelableWorker`.

```rust 
use parallel_worker::prelude::*;

fn main() {
    let mut worker = OrderedWorker::new(|n: u64| {
        sleep(std::time::Duration::from_millis(n % 3));
        n    
    });
    worker.add_task(1);
    worker.add_task(2);
    worker.add_tasks(3..=10);
    assert_eq!(worker.get_blocking(), Some(1));
    assert_eq!(worker.get_vec_blocking(), (2..=10).collect::<Vec<_>>());
}
```
