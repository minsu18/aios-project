//! CPU abstraction — task scheduling, yield

/// Create a new task (placeholder)
pub fn task_create(entry: fn(), _stack_size: usize) -> Result<u64, &'static str> {
    let _ = entry;
    Err("not implemented")
}

/// Yield to scheduler (placeholder)
pub fn yield_now() {
    // TODO: HAL implementation — switch to next task
}
