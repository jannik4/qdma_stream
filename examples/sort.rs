use anyhow::{bail, ensure, Ok, Result};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::{io::Write, thread};

const LEN: usize = 4096;

fn main() -> Result<()> {
    run_test(0, 42)?;
    run_test(0, 64)?;
    run_test(0, 17)?;

    Ok(())
}

fn run_test(queue: usize, seed: u64) -> Result<()> {
    let data = TestPacket::random_data(seed);

    let threads = vec![
        thread::spawn(move || write_to_queue(queue, data)),
        thread::spawn(move || read_from_queue(queue, data)),
    ];

    for t in threads {
        t.join().unwrap()?;
    }

    Ok(())
}

fn write_to_queue(queue: usize, data: TestPacket) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
    )?;

    stream.write_all(&data.0)?;
    stream.flush()?;

    Ok(())
}

fn read_from_queue(queue: usize, data: TestPacket) -> Result<()> {
    let mut sorted = data;
    sorted.sort();

    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let received = stream.next_packet()?;

    if received != data.0 {
        println!("data:");
        dbg_packet(&data.0);
        println!("\nsorted:");
        dbg_packet(&sorted.0);
        println!("\nreceived:");
        dbg_packet(received);

        bail!("packet mismatch");
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestPacket([u8; LEN]);

impl TestPacket {
    fn random_data(seed: u64) -> Self {
        let mut state = u64::max(1, seed);
        Self(std::array::from_fn(|_| {
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
        }))
    }

    fn sort(&mut self) {
        // Transmute to &mut [[u32; 8]]
        assert_eq!(self.0.len() % 32, 0);
        let data = unsafe { &mut *(self.0.as_mut_ptr() as *mut [[u32; 8]; LEN / 32]) };

        // Sort
        data.sort_by(|a, b| a[7].cmp(&b[7]));
    }
}

fn dbg_packet(packet: &[u8]) {
    for c in packet.chunks(32) {
        println!("{:?}", c);
    }
}
