use parallel_worker::Worker;

fn main() {
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
}