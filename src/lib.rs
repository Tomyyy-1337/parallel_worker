//! # Parallel Worker
//!
//! This crate provides a simple interface for running tasks in parallel.
//! The `Worker` struct is used to dispatch tasks to worker threads and collect the results.
//! You can wait for results or reviece currently available results.
//!
//! ```rust
//!  use parallel_worker::{State, Worker};
//!
//!  fn main() {
//!     let mut worker = Worker::new(worker_function);
//!
//!     worker.add_task(1);
//!     worker.add_task(2);
//!
//!     let first_result = worker.wait_for_result();
//!     assert_eq!(first_result, 42);
//!     
//!     worker.add_tasks(3..10);
//!
//!     let results = worker.wait_for_all_results();
//!     assert!(results.len() == 8);
//! }
//!
//! fn worker_function(task: u64, _state: &State) -> Option<u64> {
//!     Some(42)
//! }
//! ```
//!
//! ## Tasks can be canceled
//! ```rust
//! # use parallel_worker::{check_if_cancelled, State, Worker};
//! # use std::{thread::sleep, time::Duration};
//! fn main() {
//!     let mut worker = Worker::new(worker_function);
//!
//!     worker.add_task(1);
//!
//!     worker.clear_queue();
//!
//!     assert_eq!(worker.wait_for_all_results(), vec![]);
//! }
//!
//! fn worker_function(task: u64, state: &State) -> Option<u64> {
//!     for i in 0.. {
//!         sleep(Duration::from_secs(1)); // Do some work
//!         check_if_cancelled!(state); // Check if the task has been canceled
//!     }
//!     Some(42)
//! }
//!

mod task_queue;

mod worker_state;
pub use crate::worker_state::State;

mod worker;
pub use crate::worker::Worker;
