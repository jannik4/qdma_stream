use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{ctl, CardToHostStream, CardToHostStreamAsync, HostToCardStream};
use std::{io::Write, thread, time::Instant};

#[monoio::main]
async fn main() -> Result<()> {
    Test::new(0, 1, 1000, false).run().await?;
    Test::new(0, 1, 100_000, false).run().await?;
    Test::new(0, 4, 100_000, false).run().await?;

    Test::new(0, 1, 1000, true).run().await?;
    Test::new(0, 4, 1000, true).run().await?;

    Ok(())
}

struct Test {
    queue_start: usize,
    queue_count: usize,
    num_packets: usize,
    is_async: bool,
    needs_clean_up: bool,
}

impl Test {
    fn new(queue_start: usize, queue_count: usize, num_packets: usize, is_async: bool) -> Self {
        Self {
            queue_start,
            queue_count,
            num_packets,
            is_async,
            needs_clean_up: false,
        }
    }

    async fn run(mut self) -> Result<()> {
        self.needs_clean_up = true;

        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");

        println!("----- STARTING TEST -----");
        println!("Queue start: {}", self.queue_start);
        println!("Queue count: {}", self.queue_count);
        println!("Number of packets: {}", self.num_packets);
        println!("Is async: {}", self.is_async);

        println!("----- STARTING QUEUES -----");
        for dir in [ctl::QueueDir::C2h, ctl::QueueDir::H2c] {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                ctl::queue_add("qdmac1000", queue, dir)?;
                ctl::queue_start("qdmac1000", queue, dir)?;
            }
        }

        let mut threads = Vec::new();
        let mut tasks = Vec::new();
        for queue in self.queue_start..self.queue_start + self.queue_count {
            threads.push(thread::spawn(move || {
                write_to_queue(queue, self.num_packets)
            }));
            if self.is_async {
                tasks.push(monoio::spawn(async move {
                    read_from_queue_async(queue, self.num_packets).await
                }));
            } else {
                threads.push(thread::spawn(move || {
                    read_from_queue(queue, self.num_packets)
                }));
            }
        }

        monoio::blocking::spawn_blocking(move || {
            for t in threads {
                t.join().unwrap()?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .unwrap()?;
        for t in tasks {
            t.await?;
        }

        self.stop_queues()?;

        println!("----- SUCCESS -----");

        Ok(())
    }

    fn stop_queues(mut self) -> Result<()> {
        println!("----- STOPPING QUEUES -----");
        for dir in [ctl::QueueDir::C2h, ctl::QueueDir::H2c] {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                ctl::queue_stop("qdmac1000", queue, dir)?;
                ctl::queue_del("qdmac1000", queue, dir)?;
            }
        }

        self.needs_clean_up = false;

        Ok(())
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        if !self.needs_clean_up {
            return;
        }

        println!("----- CLEANING UP -----");
        for dir in [ctl::QueueDir::C2h, ctl::QueueDir::H2c] {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                if let Err(err) = ctl::queue_stop("qdmac1000", queue, dir) {
                    eprintln!("queue_stop error: {:?}", err);
                }
                if let Err(err) = ctl::queue_del("qdmac1000", queue, dir) {
                    eprintln!("queue_del error: {:?}", err);
                }
            }
        }
    }
}

fn write_to_queue(queue: usize, num_packets: usize) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
    )?;

    let mut test_packet = TestPacket::new();

    let start = Instant::now();
    for packet in 0..num_packets {
        test_packet.set_from_queue_and_packet(queue, packet);
        stream.write_all(&test_packet.0)?;
    }
    stream.flush()?;
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = num_packets * test_packet.0.len();
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

fn read_from_queue(queue: usize, num_packets: usize) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let mut test_packet = TestPacket::new();

    let start = Instant::now();
    for packet in 0..num_packets {
        test_packet.set_from_queue_and_packet(queue, packet);
        let received = stream.next_packet()?;
        if received != test_packet.0 {
            anyhow::bail!("queue({}): packet {} mismatch", queue, packet);
        }
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = num_packets * CardToHostStream::PACKET_SIZE;
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

async fn read_from_queue_async(queue: usize, num_packets: usize) -> Result<()> {
    let mut stream = CardToHostStreamAsync::new(format!("/dev/qdmac1000-ST-{}", queue)).await?;

    let start = Instant::now();
    for _ in 0..num_packets {
        stream.next_packet().await?;
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = num_packets * CardToHostStreamAsync::PACKET_SIZE;
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

struct TestPacket(Vec<u8>);

impl TestPacket {
    fn new() -> Self {
        Self(vec![0; 4096])
    }

    fn set_from_queue_and_packet(&mut self, queue: usize, packet: usize) {
        self.0[0..8].copy_from_slice(&u64::to_ne_bytes(queue as u64));
        self.0[8..16].copy_from_slice(&u64::to_ne_bytes(packet as u64));
    }
}
