use anyhow::{bail, Ok, Result};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::thread;

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

    let threads = vec![
        thread::spawn({
            let data = data.clone();
            move || write_to_queue(queue, data)
        }),
        thread::spawn(move || read_from_queue(queue, data)),
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

    stream.write_remaining(&data.0)?;

    Ok(())
}

fn read_from_queue(queue: usize, data: TestData) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let mut received = Vec::new();
    loop {
        let (is_last, packet) = stream.next_packet_protocol()?;
        received.extend_from_slice(packet);
        dbg!((is_last, packet.len()));
        if is_last {
            break;
        }
    }

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
