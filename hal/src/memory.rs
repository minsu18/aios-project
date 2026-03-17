//! Memory abstraction — alloc, free, mmap

/// Allocate memory (placeholder)
pub fn alloc(size: usize) -> Option<*mut u8> {
    // TODO: HAL implementation — kernel allocator
    let _ = size;
    None
}

/// Free memory (placeholder)
pub fn free(ptr: *mut u8) {
    let _ = ptr;
}

/// Map memory region (placeholder)
pub fn mmap(_addr: Option<*mut u8>, _len: usize, _prot: u32) -> Result<*mut u8, &'static str> {
    Err("not implemented")
}
