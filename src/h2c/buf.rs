use crate::{
    util::{mem_aligned, mem_aligned_free},
    ALIGN,
};
use anyhow::{ensure, Result};
use std::{
    io::{self, Write},
    ptr::{self, NonNull},
};

unsafe impl Send for Buf {}

pub struct Buf {
    ptr: NonNull<u8>,
    capacity: usize,
    len: usize,
}

impl Buf {
    pub fn new(capacity: usize) -> Result<Self> {
        ensure!(capacity % ALIGN == 0);
        let ptr = mem_aligned(capacity, ALIGN)?;
        Ok(Self {
            ptr,
            capacity,
            len: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn write_into<W: Write>(&mut self, mut writer: W) -> io::Result<()> {
        if self.len == 0 {
            return Ok(());
        }

        // Write aligned part of the buffer
        let len = self.len / ALIGN * ALIGN;
        let slice = unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), len) };
        writer.write_all(slice)?;

        // Write rest of the buffer
        if len < self.len {
            let slice =
                unsafe { std::slice::from_raw_parts(self.ptr.as_ptr().add(len), self.len - len) };
            writer.write_all(slice)?;
        }

        // Reset buffer
        self.len = 0;

        Ok(())
    }
}

impl Write for Buf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = usize::min(buf.len(), self.capacity - self.len);

        unsafe {
            ptr::copy_nonoverlapping(buf.as_ptr(), self.ptr.as_ptr().add(self.len), count);
        }
        self.len += count;

        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe {
            mem_aligned_free(self.ptr.as_ptr(), self.capacity, ALIGN);
        }
    }
}
