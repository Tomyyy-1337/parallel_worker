use std::thread::sleep;

use parallel_worker::{Worker, check_if_cancelled};

#[test]
fn test_worker_base() {
    let mut worker = Worker::new(|n, _s| Some(n));
    assert_eq!(worker.num_pending_tasks(), 0);

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = worker.get_vec_blocking();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_get_if_ready() {
    let mut worker = Worker::new(|n, _s| Some(n));

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
    let mut worker = Worker::new(|n, _s| Some(n));

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
    let mut worker = Worker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = Vec::new();
    while worker.num_pending_tasks() > 0 {
        results.push(worker.get_blocking());
    }
    results.sort_unstable();
    assert_eq!(results, vec![Some(1), Some(2), Some(3)]);

    assert_eq!(worker.get_blocking(), None);

    worker.add_task(4);
    worker.add_task(5);

    let mut results = Vec::new();
    while worker.num_pending_tasks() > 0 {
        results.push(worker.get_blocking());
    }

    results.sort_unstable();
    assert_eq!(results, vec![Some(4), Some(5)]);
}

#[test]
fn test_wait_for_all_results() {
    let mut worker = Worker::new(|n, _s| if n % 2 == 0 { Some(n) } else { None });

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
    let mut worker = Worker::with_num_threads(4, |n, _s| Some(n));

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
    let mut worker = Worker::with_num_threads(4, |n, _s| Some(n));

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
    let mut worker = Worker::new(|n, _s| if n % 2 == 0 { Some(n) } else { None });

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
    let mut worker = Worker::new(|n, _s| Some(n));

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
    let mut worker = Worker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results: Vec<i32> = worker.get_iter_blocking().collect();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_cancel() {
    let mut worker: Worker<i32, ()> = Worker::new(|_, s| {
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
    worker.clear_queue();
    assert_eq!(worker.get(), None);

    assert_eq!(worker.get_vec_blocking(), vec![]);
    assert!(start.elapsed() < std::time::Duration::from_secs(1));

    assert_eq!(worker.get_iter().next(), None);
    assert_eq!(worker.get_iter_blocking().next(), None);
}

#[test]
fn test_long_running_worker() {
    let mut worker = Worker::new(|n, _s| {
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
