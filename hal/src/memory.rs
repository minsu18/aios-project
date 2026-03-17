//! Memory abstraction

/// Allocate memory (placeholder)
pub fn alloc(size: usize) -> Option<*mut u8> {
    // TODO: HAL implementation
    let _ = size;
    None
}

/// Free memory (placeholder)
pub fn free(ptr: *mut u8) {
    let _ = ptr;
}
