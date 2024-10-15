use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::HostToCardStream;
use std::{io::Write, time::Instant};

fn main() -> Result<()> {
    let mut stream = HostToCardStream::new(
        "/dev/qdmac1000-ST-0",
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_secs(1),
    )?;

    let buf = vec![0; 4096];
    let count = 100_000;

    let start = Instant::now();
    for _ in 0..count {
        stream.write_all(&buf)?;
    }
    stream.flush()?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = count * buf.len();
    let speed = bytes as f64 / elapsed;
    println!(
        "writen {} bytes in {:.6} seconds @ {}/s",
        bytes,
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    Ok(())
}
