use std::thread::sleep;

use parallel_worker::Worker;

#[test]
fn test_worker_base() {
    let mut worker = Worker::new(|n, _s| Some(n));
    assert_eq!(worker.num_pending_tasks(), 0);

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut results = worker.wait_for_all_results();
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
        match worker.get_if_ready() {
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
        match worker.receive_all_results().as_slice() {
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
fn test_wait_for_all_results() {
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
fn test_receive_results_in_buffer_blocking() {
    let mut worker = Worker::with_num_threads(4, |n, _s| Some(n));

    worker.add_task(1);
    worker.add_task(2);
    worker.add_task(3);

    let mut buffer = [0; 2];
    let mut results = vec![];

    while results.len() < 3 {
        let num_results = worker.receive_results_in_buffer_blocking(&mut buffer);
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
    let num_results = worker.receive_results_in_buffer_blocking(&mut results);
    
    assert!([1,2,3].contains(&results[0]) && [1,2,3].contains(&results[1]));
    assert_eq!(num_results, 2);

    let num_results = worker.receive_results_in_buffer_blocking(&mut results);
    assert!([1,2,3].contains(&results[0]));
    assert_eq!(num_results, 1);
}

#[test]
fn test_optional_return() {
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

    let results = worker.wait_for_result();
    assert_eq!(results, Some(2));
    assert!(worker.wait_for_result().is_none());

    let results = worker.wait_for_all_results();
    assert_eq!(results, vec![]);

    worker.add_task(4);
    worker.add_task(5);
    worker.add_task(6);

    let mut results = worker.wait_for_all_results();
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

    let results = worker.wait_for_all_results();
    assert_eq!(results.len(), 3000);
}