use super::CardToHostStreamAsync;
use crate::util::{mem_aligned, mem_aligned_free};
use anyhow::Result;
use monoio::buf::IoBufMut;
use std::ptr::NonNull;

pub struct Buf {
    ptr: NonNull<u8>,
}

impl Buf {
    pub fn new() -> Result<Self> {
        let ptr = mem_aligned(
            CardToHostStreamAsync::PACKET_SIZE,
            CardToHostStreamAsync::ALIGN,
        )?;
        let slice = unsafe {
            std::slice::from_raw_parts_mut(ptr.as_ptr(), CardToHostStreamAsync::PACKET_SIZE)
        };
        slice.copy_from_slice(&[0; CardToHostStreamAsync::PACKET_SIZE]);

        Ok(Self { ptr })
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), CardToHostStreamAsync::PACKET_SIZE) }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe {
            mem_aligned_free(
                self.ptr.as_ptr(),
                CardToHostStreamAsync::PACKET_SIZE,
                CardToHostStreamAsync::ALIGN,
            );
        }
    }
}

unsafe impl IoBufMut for Buf {
    fn write_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    fn bytes_total(&mut self) -> usize {
        CardToHostStreamAsync::PACKET_SIZE
    }

    unsafe fn set_init(&mut self, _pos: usize) {
        // Is initialized in `new`
    }
}
