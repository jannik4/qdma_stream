use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::CardToHostStream;
use std::{thread, time::Instant};

fn main() -> Result<()> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or("4".to_string())
        .parse::<u32>()?;

    let mut threads = Vec::new();
    for queue in 0..count {
        threads.push(thread::spawn(move || {
            read_from_queue(queue).unwrap();
        }));
    }

    for t in threads {
        t.join().unwrap();
    }

    Ok(())
}

fn read_from_queue(queue: u32) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let mut buf = vec![0; 4096].try_into().unwrap();
    let count = 100_000;

    let start = Instant::now();
    for _ in 0..count {
        stream.next_packet(&mut buf)?;
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = count * buf.len();
    let speed = bytes as f64 / elapsed;
    println!(
        "queue({}): read {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    Ok(())
}
