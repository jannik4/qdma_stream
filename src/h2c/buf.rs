use crate::util::{mem_aligned, mem_aligned_free};
use anyhow::{ensure, Result};
use std::{
    io::{self, Write},
    ptr::{self, NonNull},
};

const ALIGN: usize = 4096;

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
        let len = self.len / ALIGN * ALIGN;
        if len == 0 {
            return Ok(());
        }

        let slice = unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), len) };
        writer.write_all(slice)?;

        if len < self.len {
            unsafe {
                ptr::copy(
                    self.ptr.as_ptr().add(len),
                    self.ptr.as_ptr(),
                    self.len - len,
                );
            }
        }

        self.len -= len;

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
