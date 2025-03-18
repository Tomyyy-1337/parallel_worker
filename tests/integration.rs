use std::thread::sleep;

use parallel_worker::Worker;

#[test]
fn test_worker_base() {
    let mut worker = Worker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = worker.wait_for_all_results();
    results.sort_unstable();
    assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_iter() {
    let mut worker = Worker::new(|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = Vec::new();
    while results.len() < 3 {
        match worker.next() {
            Some(result) => results.push(result),
            None => sleep(std::time::Duration::from_millis(10)),
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
        results.push(worker.wait_for_result());
    }
    results.sort_unstable();
    assert_eq!(results, vec![Some(1), Some(2), Some(3)]);

    assert_eq!(worker.wait_for_result(), None);

    worker.add_task(4);
    worker.add_task(5);

    let mut results = Vec::new();
    while worker.num_pending_tasks() > 0 {
        results.push(worker.wait_for_result());
    }

    results.sort_unstable();
    assert_eq!(results, vec![Some(4), Some(5)]);
}


#[test]
fn test_worker_optional_result() {
    let mut worker = Worker::new(|n, _s| 
        if n % 2 == 0 {
            Some(n)
        } else {
            None
        }
    );
    
    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);


    let results = worker.wait_for_all_results();
    assert_eq!(results, vec![2]);

    let mut results = worker.wait_for_all_results();
    results.sort_unstable();
    assert_eq!(results, vec![]);

    worker.add_task(4);
    worker.add_task(5);
    worker.add_task(6);

    let mut results = worker.wait_for_all_results();
    results.sort_unstable();
    assert_eq!(results, vec![4, 6]);
}

#[test]
fn test_wait_for_task() {
    let mut worker = Worker::with_num_threads(4 ,|n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = Vec::new();
    while worker.num_pending_tasks() > 0 {
        results.push(worker.wait_for_result().unwrap());
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
    let num_results = worker.receive_results_in_buffer_blocking(&mut results);
    
    assert!([1,2,3].contains(&results[0]) && [1,2,3].contains(&results[1]));
    assert_eq!(num_results, 2);

    let num_results = worker.receive_results_in_buffer_blocking(&mut results);
    assert!([1,2,3].contains(&results[0]));
    assert_eq!(num_results, 1);
}