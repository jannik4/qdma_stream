mod buf;

use self::buf::Buf;
use anyhow::Result;
use std::{
    fs,
    io::{self, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub struct HostToCardStream {
    alive: Arc<AtomicBool>,
    stream: Arc<Mutex<Stream>>,

    flush_threshold: usize,
}

impl HostToCardStream {
    pub fn new(
        path: impl AsRef<Path>,
        capacity: usize,
        flush_threshold: usize,
        flush_interval: Duration,
    ) -> Result<Self> {
        let alive = Arc::new(AtomicBool::new(true));
        let stream = Arc::new(Mutex::new(Stream {
            buf: Buf::new(capacity)?,
            file: fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path.as_ref())?,
            last_write_to_file: Instant::now(),
        }));

        let alive_clone = Arc::clone(&alive);
        let stream_clone = Arc::clone(&stream);
        thread::spawn(move || daemon(alive_clone, stream_clone, flush_interval));

        Ok(Self {
            alive,
            stream,
            flush_threshold,
        })
    }

    /// Use this to write remaining packets and finish the stream.
    pub fn write_remaining(&mut self, remaining: &[u8]) -> io::Result<()> {
        // Calculate count of remaining packets
        let remaining_packet_count = u32::div_ceil(remaining.len() as u32, 4096);

        let mut stream = self.stream.lock().unwrap();

        // Write remaining packets count
        stream.write_remaining_packet_count(remaining_packet_count)?;

        // Write remaining data
        stream.file.write_all(remaining)?;

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

impl Write for HostToCardStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut stream = self.stream.lock().unwrap();
        let count = stream.buf.write(buf)?;
        if stream.buf.len() >= self.flush_threshold {
            stream.flush()?;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut stream = self.stream.lock().unwrap();
        stream.flush()?;
        Ok(())
    }
}

impl Drop for HostToCardStream {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.flush();
    }
}

struct Stream {
    buf: Buf,
    file: fs::File,
    last_write_to_file: Instant,
}

impl Stream {
    fn flush(&mut self) -> io::Result<()> {
        self.last_write_to_file = Instant::now();
        self.buf.write_into(&mut self.file)?;
        Ok(())
    }

    fn write_remaining_packet_count(&mut self, count: u32) -> io::Result<()> {
        // Flush existing buffer
        self.flush()?;

        // Write count of remaining packets
        self.file.write_all(&u32::to_le_bytes(count))?;

        Ok(())
    }
}

fn daemon(alive: Arc<AtomicBool>, stream: Arc<Mutex<Stream>>, flush_interval: Duration) {
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
