//! Bump allocator for kernel-rpi.
//! Prepares for llama.cpp integration (requires malloc).
//! Heap is a static array; allocations are linear, no dealloc.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 128 * 1024; // 128KB

#[repr(align(4096))]
struct Heap([u8; HEAP_SIZE]);

static HEAP: Heap = Heap([0; HEAP_SIZE]);
static BUMP: AtomicUsize = AtomicUsize::new(0);

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut bump = BUMP.load(Ordering::Relaxed);
        bump = (bump + align - 1) & !(align - 1);
        let new_bump = bump + size;
        if new_bump > HEAP_SIZE {
            return core::ptr::null_mut();
        }
        BUMP.store(new_bump, Ordering::Relaxed);
        HEAP.0.as_ptr().add(bump) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* Bump allocator: no-op */
    }
}
