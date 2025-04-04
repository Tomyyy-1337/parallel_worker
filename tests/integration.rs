use std::thread::sleep;

use parallel_worker::prelude::*;

#[test]
fn basic_worker() {
    let mut worker = BasicWorker::new(|n: u64| n);

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let s: u64 = worker.get_iter_blocking().sum();
    assert_eq!(s, 6);

    worker.add_tasks(0..10);

    assert_eq!(worker.get_iter_blocking().count(), 10);
    assert_eq!(worker.get_iter_blocking().count(), 0);
}

#[test]
fn test_worker_base() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));
    assert_eq!(worker.pending_tasks(), 0);

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_get_if_ready() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = Vec::new();
    while results.len() < 3 {
        match worker.get() {
            Some(result) => results.push(result),
            None => sleep(std::time::Duration::from_millis(10)),
        }
    }
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_receive_all_results() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results: Vec<i32> = Vec::new();
    while results.len() < 3 {
        match worker.get_vec().as_slice() {
            [] => sleep(std::time::Duration::from_millis(10)),
            a @ [..] => results.extend(a),
        }
    }
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_wait_for_result() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);

    assert_eq!(worker.get_blocking(), None);

    worker.add_task(4);
    worker.add_task(5);

    let mut results = worker.get_vec_blocking();

    results.sort_unstable();
    assert_eq!(results, vec![4, 5]);
}

#[test]
fn test_wait_for_all_results() {
    let mut worker = CancelableWorker::new(|n, _s| if n % 2 == 0 { Some(n) } else { None });

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let results = worker.get_vec_blocking();
    assert_eq!(results, vec![2]);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![]);

    worker.add_task(4);
    worker.add_task(5);
    worker.add_task(6);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![4, 6]);
}

#[test]
fn test_receive_results_in_buffer_blocking() {
    let mut worker = CancelableWorker::with_num_threads(4, |n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut buffer = [0; 2];
    let mut results = vec![];

    while results.len() < 3 {
        let num_results = worker.get_buffered_blocking(&mut buffer);
        results.extend_from_slice(&buffer[..num_results]);
    }

    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_receive_results_in_buffer() {
    let mut worker = CancelableWorker::with_num_threads(4, |n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = [0; 2];
    let num_results = worker.get_buffered_blocking(&mut results);

    assert!([1, 2, 3].contains(&results[0]) && [1, 2, 3].contains(&results[1]));
    assert_eq!(num_results, 2);

    let num_results = worker.get_buffered_blocking(&mut results);
    assert!([1, 2, 3].contains(&results[0]));
    assert_eq!(num_results, 1);
}

#[test]
fn test_optional_return() {
    let mut worker = CancelableWorker::new(|n, _s| if n % 2 == 0 { Some(n) } else { None });

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let results = worker.get_blocking();
    assert_eq!(results, Some(2));
    assert!(worker.get().is_none());

    let results = worker.get_vec_blocking();
    assert_eq!(results, vec![]);

    worker.add_task(4);
    worker.add_task(5);
    worker.add_task(6);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![4, 6]);
}

#[test]
fn test_many_tasks() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));

    for i in 0..1000 {
        worker.add_task(i);
    }
    worker.add_tasks(0..1000);
    for i in 0..1000 {
        worker.add_task(i);
    }

    let results = worker.get_vec_blocking();
    assert_eq!(results.len(), 3000);
}

#[test]
fn test_get_iter() {
    let mut worker = CancelableWorker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results: Vec<i32> = worker.get_iter_blocking().collect();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_cancel() {
    let mut worker: CancelableWorker<i32, ()> = CancelableWorker::new(|_, s| {
        loop {
            check_if_cancelled!(s);
            sleep(std::time::Duration::from_millis(10));
        }
    });

    let start = std::time::Instant::now();
    worker.add_task(10);
    worker.add_task(20);
    worker.add_task(30);

    assert_eq!(worker.get(), None);
    worker.cancel_tasks();
    assert_eq!(worker.get(), None);

    assert_eq!(worker.get_vec_blocking(), vec![]);
    assert!(start.elapsed() < std::time::Duration::from_secs(1));

    assert_eq!(worker.get_iter().next(), None);
    assert_eq!(worker.get_iter_blocking().next(), None);
}

#[test]
fn test_long_running_worker() {
    let mut worker = CancelableWorker::new(|n, _s| {
        sleep(std::time::Duration::from_millis(n));
        Some(n)
    });

    let mut results = Vec::new();
    for i in 0..100 {
        worker.add_task(i);
        worker.add_task(i);

        if let Some(result) = worker.get() {
            results.push(result);
        }
    }

    results.extend(worker.get_iter_blocking());

    assert_eq!(results.len(), 200);
}

#[test]
fn test_cancel_high_load() {
    use std::{thread::sleep, time::Duration};

    let mut worker = CancelableWorker::new(|n: u64, _s: &State| {
        sleep(Duration::from_millis(n));
        Some(n)
    });

    for _ in 0..100 {
        worker.add_task(20);
        worker.add_task(20);
        worker.add_task(20);
        worker.add_task(20);

        worker.cancel_tasks();

        assert!(worker.get_iter_blocking().next().is_none());

        worker.add_task(0);

        assert_eq!(worker.get_blocking(), Some(0));
    }
}

#[test]
fn test_cancel_count() {
    use std::{thread::sleep, time::Duration};

    let mut worker = CancelableWorker::new(|n: u64, _s: &State| {
        sleep(Duration::from_millis(20));
        Some(n)
    });

    worker.add_tasks(0..50);

    worker.cancel_tasks();

    worker.add_tasks(0..50);

    let num_results = worker.get_iter_blocking().count();

    assert_eq!(num_results, 50);

    for i in 0..100 {
        worker.add_task(i);
        worker.add_task(i);
        worker.add_task(i);
        worker.cancel_tasks();
    }
    assert!(worker.get_blocking().is_none());

    for i in 0..100 {
        worker.add_task(i);
    }
    worker.cancel_tasks();
    worker.add_task(0);

    assert_eq!(worker.get_blocking(), Some(0));
    assert!(worker.get_blocking().is_none());

    drop(worker);
}

#[test]
fn test_reset() {
    let mut worker = BasicWorker::new(|n: u64| {
        sleep(std::time::Duration::from_millis(n));
        n
    });

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);
    worker.add_task(200);
    worker.add_task(201);
    worker.add_task(202);

    sleep(std::time::Duration::from_millis(50));

    worker.reset();

    worker.add_task(203);

    assert_eq!(worker.get_blocking(), Some(203));
    assert!(worker.get_blocking().is_none());
}

#[test]
fn test_ordered_worker() {
    let mut worker = OrderedWorker::new(|n: u64| {
        sleep(std::time::Duration::from_millis(n % 3));
        n
    });

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let results = worker.get_vec_blocking();
    assert_eq!(results, vec![1, 2, 3]);

    worker.add_tasks(0..10000);

    let results = worker.get_vec_blocking();
    assert_eq!(results, (0..10000).collect::<Vec<_>>());
}

#[test]
fn test_ordered_worker_cancel() {
    let mut worker = OrderedWorker::with_num_threads(2, |n: u64| {
        sleep(std::time::Duration::from_millis(n));
        n
    });

    worker.add_task(100);
    worker.add_task(101);
    worker.add_task(102);

    sleep(std::time::Duration::from_millis(50));
    worker.cancel_tasks();

    let result = worker.get_vec_blocking();
    assert_eq!(result, vec![100, 101]);
    assert_eq!(worker.get_blocking(), None);

    worker.add_task(103);
    worker.add_task(104);
    worker.add_task(105);

    let results = worker.get_vec_blocking();
    assert_eq!(results, vec![103, 104, 105]);
    assert_eq!(worker.get_blocking(), None);

    worker.add_task(106);
    assert_eq!(worker.get_blocking(), Some(106));

    worker.add_tasks(100..1000);

    sleep(std::time::Duration::from_millis(50));

    worker.cancel_tasks();

    let results = worker.get_vec_blocking();
    assert_eq!(results, vec![100, 101]);
    assert_eq!(worker.get_blocking(), None);

    worker.add_tasks(107..110);
    let results = worker.get_vec_blocking();

    assert_eq!(results, (107..110).collect::<Vec<_>>());
}
