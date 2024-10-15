use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::HostToCardStream;
use std::{io::Write, thread, time::Instant};

fn main() -> Result<()> {
    let mut threads = Vec::new();
    for queue in 0..4 {
        threads.push(thread::spawn(move || {
            write_to_queue(queue).unwrap();
        }));
    }

    for t in threads {
        t.join().unwrap();
    }

    Ok(())
}

fn write_to_queue(queue: u32) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
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
        "queue({}): writen {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    Ok(())
}
