mod buf;

use self::buf::Buf;
use anyhow::Result;
use std::{
    io::{self, Read, Write},
    time::Instant,
};

pub struct HostToCardStream<F: Write> {
    buf: Buf,
    file: F,
    last_write_to_file: Instant,
    flush_threshold: usize,
}

impl<F> HostToCardStream<F>
where
    F: Write + 'static,
{
    pub fn new(file: F, capacity: usize, flush_threshold: usize) -> Result<Self> {
        Ok(Self {
            buf: Buf::new(capacity)?,
            file,
            last_write_to_file: Instant::now(),
            flush_threshold,
        })
    }
}

impl<F> HostToCardStream<F>
where
    F: Write,
{
    pub fn write_complete_stream(&mut self, mut buf: impl Read, length: usize) -> io::Result<()> {
        if length == 0 {
            panic!("length is zero");
        }

        self.write_remaining_packet_count(usize::div_ceil(length, 4096) as u32)?;
        let written = io::copy(&mut buf, self)?;
        self.flush()?;

        if written != length as u64 {
            panic!("written bytes does not match length");
        }

        Ok(())
    }

    /// Use this to write remaining packets and finish the stream.
    ///
    /// # Panics
    ///
    /// Panics if `remaining` is empty.
    pub fn write_remaining(&mut self, remaining: &[u8]) -> io::Result<()> {
        if remaining.is_empty() {
            panic!("remaining data is empty");
        }

        // Calculate count of remaining packets
        let remaining_packet_count = usize::div_ceil(remaining.len(), 4096) as u32;

        // Write remaining packets count
        self.write_remaining_packet_count(remaining_packet_count)?;

        // Write remaining data
        self.write_all(remaining)?;
        self.flush()?;

        Ok(())
    }

    /// Use this to write the count of remaining packets. This is useful when you know early on
    /// how many packets you will be writing. The stream will be finished when the count of packets
    /// is reached.
    pub fn write_remaining_packet_count(&mut self, count: u32) -> io::Result<()> {
        // Flush existing buffer
        self.flush()?;

        // Write count of remaining packets
        self.buf.write_all(&u32::to_le_bytes(count))?;
        self.flush()?;

        Ok(())
    }
}

impl<F> Write for HostToCardStream<F>
where
    F: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = self.buf.write(buf)?;
        if self.buf.len() >= self.flush_threshold {
            self.flush()?;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.last_write_to_file = Instant::now();
        self.buf.write_into(&mut self.file)?;
        Ok(())
    }
}

impl<F> Drop for HostToCardStream<F>
where
    F: Write,
{
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
