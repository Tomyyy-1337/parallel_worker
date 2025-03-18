use std::sync::{Arc, Mutex};

/// Check if the task has been canceled and return None if it has.
/// Can be used inside the worker function to check if the task has been canceled.
/// 
/// ## Example usage:
/// ```rust ignore
/// fn worker_function(task: T, state: &State) -> Option<R> {
///     while work_not_done {
///         check_if_cancelled!(state);   
///         do_work();
///     }
///     Some(result)
/// }
/// ``` 
///
/// ## Shorthand for:
/// ```rust ignore
/// if state.is_cancelled() {
///    return None;
/// }
/// ```
#[macro_export]
macro_rules! check_if_cancelled {
    ($state:expr) => {
        if $state.is_cancelled() {
            return None;
        }
    };
}

enum WorkerState {
    Running,
    Canceled,
    Waiting,
}

/// State of the worker. Used to check if the task has been canceled.
/// Check if the task has been canceled using the `is_cancelled` method.
/// Or use the `check_if_cancelled!` macro to check and return None from the worker function.
pub struct State {
    state: Arc<Mutex<WorkerState>>,
}

impl State {
    pub(crate) fn new() -> State {
        State {
            state: Arc::new(Mutex::new(WorkerState::Waiting)),
        }
    }

    pub(crate) fn set_waiting(&self) {
        *self.state.lock().unwrap() = WorkerState::Waiting;
    }

    pub(crate) fn set_running(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match *state {
            WorkerState::Waiting => {
                *state = WorkerState::Running;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            WorkerState::Running => *state = WorkerState::Canceled,
            _ => (),
        }
    }

    /// Returns true if the task has been canceled. The result
    /// of the worker will be ignored. Use this to check if the
    /// task should be canceled for long running tasks.
    pub fn is_cancelled(&self) -> bool {
        match *self.state.lock().unwrap() {
            WorkerState::Canceled => true,
            _ => false,
        }
    }
}

impl Clone for State {
    fn clone(&self) -> State {
        State {
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state() {
        let state = State::new(); 
        assert!(!state.is_cancelled());

        state.set_running(); 
        assert!(!state.is_cancelled());

        state.cancel();
        assert!(state.is_cancelled());

        state.set_waiting();
        assert!(!state.is_cancelled());

        state.cancel();
        assert!(!state.is_cancelled());
    }
}