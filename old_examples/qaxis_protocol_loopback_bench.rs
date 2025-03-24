use anyhow::{bail, Result};
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::{io::Write, thread, time::Instant};

fn main() -> Result<()> {
    let queue = std::env::args()
        .nth(1)
        .unwrap_or("0".to_string())
        .parse::<usize>()?;
    let num_bytes = std::env::args()
        .nth(2)
        .unwrap_or("4096".to_string())
        .parse::<usize>()?;

    run_test(queue, num_bytes)?;

    Ok(())
}

fn run_test(queue: usize, num_bytes: usize) -> Result<()> {
    let threads = vec![
        thread::spawn(move || write_to_queue(queue, num_bytes)),
        thread::spawn(move || read_from_queue(queue, num_bytes)),
    ];

    for t in threads {
        t.join().unwrap()?;
    }

    Ok(())
}

fn write_to_queue(queue: usize, num_bytes: usize) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
    )?;

    if num_bytes == 0 {
        bail!("num_bytes must be greater than 0");
    }

    let mut num_bytes_left = num_bytes;
    let buf = vec![0; 4096];

    let start = Instant::now();
    while num_bytes_left > 4096 {
        stream.write_all(&buf)?;
        num_bytes_left -= 4096;
    }
    stream.write_remaining(&buf[..num_bytes_left])?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = num_bytes;
    let speed = bytes as f64 / elapsed;
    println!(
        "queue({}): writen {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    Ok(())
}

fn read_from_queue(queue: usize, num_bytes: usize) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let mut count_bytes = CountBytes { count: 0 };

    let start = Instant::now();
    stream.read_complete_stream(&mut count_bytes)?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = count_bytes.count;
    let speed = bytes as f64 / elapsed;
    println!(
        "queue({}): read {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    if count_bytes.count != num_bytes {
        bail!("packet mismatch");
    }

    println!("loopback successful");

    Ok(())
}

struct CountBytes {
    count: usize,
}

impl Write for CountBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.count += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
