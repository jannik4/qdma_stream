use anyhow::{Context, Result};
use std::ptr::NonNull;

pub fn mem_aligned(size: usize, align: usize) -> Result<NonNull<u8>> {
    assert!(size > 0);
    let layout = std::alloc::Layout::from_size_align(size, align).context("invalid layout")?;
    let ptr = unsafe { std::alloc::alloc(layout) };
    NonNull::new(ptr).context("failed to allocate memory")
}

pub unsafe fn mem_aligned_free(ptr: *mut u8, size: usize, align: usize) {
    let layout = std::alloc::Layout::from_size_align(size, align).unwrap();
    unsafe { std::alloc::dealloc(ptr, layout) }
}
