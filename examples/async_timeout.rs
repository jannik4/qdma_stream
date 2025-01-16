use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{ctl, CardToHostStreamAsync, HostToCardStream};
use std::{
    io::Write,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    Test::new(0, 1, vec![1000, 500, 1000]).run()?;
    Test::new(0, 1, vec![100_000, 200_000, 100_000]).run()?;
    Test::new(0, 4, vec![100_000, 100_000, 100_000]).run()?;

    Ok(())
}

struct Test {
    queue_start: usize,
    queue_count: usize,
    num_packets: Vec<usize>,
    needs_clean_up: bool,
}

impl Test {
    fn new(queue_start: usize, queue_count: usize, num_packets: Vec<usize>) -> Self {
        Self {
            queue_start,
            queue_count,
            num_packets,
            needs_clean_up: false,
        }
    }

    fn run(mut self) -> Result<()> {
        self.needs_clean_up = true;

        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");

        println!("----- STARTING TEST -----");
        println!("Queue start: {}", self.queue_start);
        println!("Queue count: {}", self.queue_count);
        println!("Number of packets: {:?}", self.num_packets);

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
            threads.push(thread::spawn({
                let num_packets = self.num_packets.clone();
                move || write_to_queue(queue, num_packets)
            }));
            tasks.push(Box::pin({
                let num_packets = self.num_packets.clone();
                async move { read_from_queue_async(queue, num_packets).await }
            }));
        }

        monoio::start::<monoio::LegacyDriver, _>(async move {
            let tasks = tasks.into_iter().map(monoio::spawn).collect::<Vec<_>>();

            for t in tasks {
                t.await?;
            }

            Ok::<_, anyhow::Error>(())
        })?;
        for t in threads {
            t.join().unwrap()?;
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

fn write_to_queue(queue: usize, num_packets: Vec<usize>) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        Duration::from_millis(10),
    )?;

    let mut test_packet = TestPacket::new();

    for num_packets in num_packets {
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

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

async fn read_from_queue_async(queue: usize, num_packets: Vec<usize>) -> Result<()> {
    let mut stream = CardToHostStreamAsync::new(format!("/dev/qdmac1000-ST-{}", queue)).await?;

    for num_packets_expected in num_packets {
        let mut num_packets_received = 0;

        let start = Instant::now();
        loop {
            let timeout =
                monoio::time::timeout(Duration::from_secs_f32(0.5), stream.next_packet()).await;
            match timeout {
                Ok(res) => {
                    res?;
                    num_packets_received += 1;
                }
                Err(_) => break,
            }
        }
        let elapsed = start.elapsed().as_secs_f64() - 0.5;

        if num_packets_expected != num_packets_received {
            anyhow::bail!(
                "queue({}): expected {} packets, but received {}",
                queue,
                num_packets_expected,
                num_packets_received
            );
        }

        let bytes = num_packets_received * CardToHostStreamAsync::PACKET_SIZE;
        let speed = bytes as f64 / elapsed;
        println!(
            "queue({}): read {} bytes in {:.6} seconds @ {}/s",
            queue,
            ISizeFormatter::new(bytes, BINARY),
            elapsed,
            ISizeFormatter::new(speed, BINARY),
        );

        monoio::time::sleep(Duration::from_secs_f32(0.5)).await;
    }

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
