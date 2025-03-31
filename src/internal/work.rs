pub(crate) enum Work<T> {
    Task(T),
    Terminate,
}
