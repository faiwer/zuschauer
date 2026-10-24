/// What the service manager reports about one service.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unit {
    /// Why it's unhealthy: failed, stopped, not found. `None` when healthy.
    pub problem: Option<String>,
    /// How many times the manager started it again. Only grows, unless the
    /// manager resets it.
    pub restarts: u64,
}
