use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Check if the task has been canceled and return [`None`] if it has.
/// Can be used inside the worker function of a [`crate::CancelableWorker`].
///
/// Can not be used with the [`crate::BasicWorker`] or [`crate::OrderedWorker`].
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

/// State of a worker thread.
///
/// Use the [`check_if_cancelled!`] macro to check if a task has been canceld.
pub struct State {
    is_canceled: Arc<AtomicBool>,
}

impl State {
    pub(crate) fn new() -> State {
        State {
            is_canceled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn set_running(&self) {
        self.is_canceled.store(false, Ordering::Release);
    }

    pub(crate) fn cancel(&self) {
        self.is_canceled.store(true, Ordering::Release);
    }

    /// Returns true if the task has been canceled. The result
    /// of the worker will be ignored.
    pub fn is_cancelled(&self) -> bool {
        self.is_canceled.load(Ordering::Acquire)
    }
}

impl Clone for State {
    fn clone(&self) -> State {
        State {
            is_canceled: self.is_canceled.clone(),
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

        state.cancel();
        assert!(state.is_cancelled());

        state.set_running();
        assert!(!state.is_cancelled());

        state.set_running();
        assert!(!state.is_cancelled());
    }
}
