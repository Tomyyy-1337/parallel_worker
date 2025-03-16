use std::sync::{Arc, Mutex};

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

    pub(crate) fn set_running(&self) {
        *self.state.lock().unwrap() = WorkerState::Running;
    }

    pub(crate) fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            WorkerState::Running => *state = WorkerState::Canceled,
            _ => (),
        }
    }

    /// Returns true if the task has been canceled. The result
    /// of the worker will be ignored.
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
