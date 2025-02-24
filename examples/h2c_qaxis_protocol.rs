use anyhow::{bail, Result};
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::HostToCardStream;
use std::{io::Write, time::Instant};

fn main() -> Result<()> {
    let queue = std::env::args()
        .nth(1)
        .unwrap_or("0".to_string())
        .parse::<u32>()?;
    let num_bytes = std::env::args()
        .nth(2)
        .unwrap_or("4096".to_string())
        .parse::<usize>()?;

    write_to_queue(queue, num_bytes)?;

    Ok(())
}

fn write_to_queue(queue: u32, num_bytes: usize) -> Result<()> {
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
