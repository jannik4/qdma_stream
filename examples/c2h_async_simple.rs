use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use monoio::spawn;
use qdma_stream::CardToHostStreamAsync;
use std::time::Instant;

#[monoio::main]
async fn main() -> Result<()> {
    main_().await
}

async fn main_() -> Result<()> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or("4".to_string())
        .parse::<u32>()?;

    let mut tasks = Vec::new();
    for queue in 0..count {
        tasks.push(spawn(async move { read_from_queue(queue).await }));
    }

    for t in tasks {
        t.await.unwrap();
    }

    Ok(())
}

async fn read_from_queue(queue: u32) -> Result<()> {
    let mut stream = CardToHostStreamAsync::new(format!("/dev/qdmac1000-ST-{}", queue)).await?;

    let count = 100_000;

    let start = Instant::now();
    for _ in 0..count {
        stream.next_packet().await?;
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = count * CardToHostStreamAsync::PACKET_SIZE;
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
