mod buf;

use self::buf::Buf;
use anyhow::Result;
use std::{
    io::{self, Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub struct HostToCardStream<F: Write> {
    alive: Arc<AtomicBool>,
    stream: Arc<Mutex<Stream<F>>>,
}

impl<F> HostToCardStream<F>
where
    F: Write + Send + 'static,
{
    pub fn new(
        file: F,
        capacity: usize,
        flush_threshold: usize,
        flush_interval: Duration,
    ) -> Result<Self> {
        let alive = Arc::new(AtomicBool::new(true));
        let stream = Arc::new(Mutex::new(Stream {
            buf: Buf::new(capacity)?,
            file,
            last_write_to_file: Instant::now(),
            flush_threshold,
        }));

        let alive_clone = Arc::clone(&alive);
        let stream_clone = Arc::clone(&stream);
        thread::spawn(move || daemon(alive_clone, stream_clone, flush_interval));

        Ok(Self { alive, stream })
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

        let mut stream = self.stream.lock().unwrap();
        stream.write_remaining_packet_count(usize::div_ceil(length, 4096) as u32)?;
        let written = io::copy(&mut buf, &mut *stream)?;
        stream.flush()?;

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

        let mut stream = self.stream.lock().unwrap();

        // Write remaining packets count
        stream.write_remaining_packet_count(remaining_packet_count)?;

        // Write remaining data
        stream.write_all(remaining)?;
        stream.flush()?;

        Ok(())
    }

    /// Use this to write the count of remaining packets. This is useful when you know early on
    /// how many packets you will be writing. The stream will be finished when the count of packets
    /// is reached.
    pub fn write_remaining_packet_count(&mut self, count: u32) -> io::Result<()> {
        self.stream
            .lock()
            .unwrap()
            .write_remaining_packet_count(count)
    }
}

impl<F> Write for HostToCardStream<F>
where
    F: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut stream = self.stream.lock().unwrap();
        stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut stream = self.stream.lock().unwrap();
        stream.flush()
    }
}

impl<F> Drop for HostToCardStream<F>
where
    F: Write,
{
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.flush();
    }
}

struct Stream<F> {
    buf: Buf,
    file: F,
    last_write_to_file: Instant,
    flush_threshold: usize,
}

impl<F> Stream<F>
where
    F: Write,
{
    fn write_remaining_packet_count(&mut self, count: u32) -> io::Result<()> {
        // Flush existing buffer
        self.flush()?;

        // Write count of remaining packets
        self.buf.write_all(&u32::to_le_bytes(count))?;
        self.flush()?;

        Ok(())
    }
}

impl<F> Write for Stream<F>
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

fn daemon<F>(alive: Arc<AtomicBool>, stream: Arc<Mutex<Stream<F>>>, flush_interval: Duration)
where
    F: Write,
{
    while alive.load(Ordering::Relaxed) {
        let mut stream = stream.lock().unwrap();
        let current = Instant::now().duration_since(stream.last_write_to_file);
        match flush_interval.checked_sub(current) {
            Some(remaining) => {
                drop(stream);
                thread::sleep(remaining);
            }
            None => {
                if let Err(err) = stream.flush() {
                    eprintln!("failed to flush: {}", err);
                }
                drop(stream);
                thread::sleep(flush_interval);
            }
        }
    }
}
