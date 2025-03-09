use anyhow::{bail, Ok, Result};
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::{thread, time::Instant};

fn main() -> Result<()> {
    let queue = std::env::args()
        .nth(1)
        .unwrap_or("0".to_string())
        .parse::<usize>()?;
    let num_bytes = std::env::args()
        .nth(2)
        .unwrap_or("4096".to_string())
        .parse::<usize>()?;
    let seed = std::env::args()
        .nth(3)
        .unwrap_or("0".to_string())
        .parse::<u64>()?;

    run_test(queue, num_bytes, seed)?;

    Ok(())
}

fn run_test(queue: usize, num_bytes: usize, seed: u64) -> Result<()> {
    let data = TestData::random_data(num_bytes, seed);
    let received = Vec::with_capacity(data.0.len());

    let threads = vec![
        thread::spawn({
            let data = data.clone();
            move || write_to_queue(queue, data)
        }),
        thread::spawn(move || read_from_queue(queue, data, received)),
    ];

    for t in threads {
        t.join().unwrap()?;
    }

    Ok(())
}

fn write_to_queue(queue: usize, data: TestData) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
    )?;

    let start = Instant::now();
    stream.write_remaining(&data.0)?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = data.0.len();
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

fn read_from_queue(queue: usize, data: TestData, mut received: Vec<u8>) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let start = Instant::now();
    stream.read_complete_protocol(&mut received)?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = received.len();
    let speed = bytes as f64 / elapsed;
    println!(
        "queue({}): read {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    if received != data.0 {
        println!("data:");
        dbg_packet(&data.0);
        println!("\nreceived:");
        dbg_packet(&received);

        bail!("packet mismatch");
    }

    println!("loopback successful");

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestData(Vec<u8>);

impl TestData {
    fn random_data(num_bytes: usize, seed: u64) -> Self {
        let mut state = u64::max(1, seed);
        Self(
            (0..num_bytes)
                .map(|_| {
                    // Xorshift64*
                    let next = {
                        let mut x = state;
                        x ^= x >> 12;
                        x ^= x << 25;
                        x ^= x >> 27;
                        state = x;
                        x.wrapping_mul(2685821657736338717)
                    };

                    next as u8
                })
                .collect(),
        )
    }
}

fn dbg_packet(packet: &[u8]) {
    let packet = &packet[..usize::min(packet.len(), 4096)];
    for c in packet.chunks(32) {
        println!("{:?}", c);
    }
}
