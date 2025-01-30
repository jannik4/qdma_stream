use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{ctl, CardToHostStream, HostToCardStream};
use std::{io::Write, thread, time::Instant};

fn main() -> Result<()> {
    Bench::new(Some(ctl::QueueDir::H2c), false, 0, 1, 200_000).run()?;
    Bench::new(Some(ctl::QueueDir::H2c), false, 0, 4, 200_000).run()?;

    Bench::new(Some(ctl::QueueDir::C2h), false, 0, 1, 200_000).run()?;
    Bench::new(Some(ctl::QueueDir::C2h), false, 0, 4, 200_000).run()?;

    Bench::new(Some(ctl::QueueDir::C2h), true, 0, 1, 200_000).run()?;
    Bench::new(Some(ctl::QueueDir::C2h), true, 0, 4, 200_000).run()?;

    Bench::new(None, false, 0, 1, 200_000).run()?;
    Bench::new(None, false, 0, 4, 200_000).run()?;

    Bench::new(None, true, 0, 1, 200_000).run()?;
    Bench::new(None, true, 0, 4, 200_000).run()?;

    Ok(())
}

struct Bench {
    dir: Option<ctl::QueueDir>,
    with_ctrl_sequence: bool,
    queue_start: usize,
    queue_count: usize,
    num_packets: usize,
    needs_clean_up: bool,
}

impl Bench {
    fn new(
        dir: Option<ctl::QueueDir>,
        with_ctrl_sequence: bool,
        queue_start: usize,
        queue_count: usize,
        num_packets: usize,
    ) -> Self {
        Self {
            dir,
            with_ctrl_sequence,
            queue_start,
            queue_count,
            num_packets,
            needs_clean_up: false,
        }
    }

    fn directions(&self) -> &'static [ctl::QueueDir] {
        match self.dir {
            Some(ctl::QueueDir::C2h) => &[ctl::QueueDir::C2h],
            Some(ctl::QueueDir::H2c) => &[ctl::QueueDir::H2c],
            None => &[ctl::QueueDir::C2h, ctl::QueueDir::H2c],
        }
    }

    fn run(mut self) -> Result<()> {
        self.needs_clean_up = true;

        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");
        println!("----------------------------------------------------------------");

        println!("----- STARTING BENCH -----");
        match self.dir {
            Some(dir) => println!("Direction: {}", dir.as_str()),
            None => println!("Direction: both"),
        }
        println!("With control sequence: {}", self.with_ctrl_sequence);
        println!("Queue start: {}", self.queue_start);
        println!("Queue count: {}", self.queue_count);
        println!("Number of packets: {}", self.num_packets);

        println!("----- STARTING QUEUES -----");
        for dir in self.directions() {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                ctl::queue_add("qdmac1000", queue, *dir)?;
                ctl::queue_start("qdmac1000", queue, *dir)?;
            }
        }

        let mut threads = Vec::new();
        for queue in self.queue_start..self.queue_start + self.queue_count {
            if self.dir.is_none() || self.dir == Some(ctl::QueueDir::C2h) {
                threads.push(thread::spawn(move || match self.with_ctrl_sequence {
                    true => read_from_queue::<true>(queue, self.num_packets),
                    false => read_from_queue::<false>(queue, self.num_packets),
                }));
            }
            if self.dir.is_none() || self.dir == Some(ctl::QueueDir::H2c) {
                threads.push(thread::spawn(move || {
                    write_to_queue(queue, self.num_packets)
                }));
            }
        }

        for t in threads {
            t.join().unwrap()?;
        }

        self.stop_queues()?;

        println!("----- SUCCESS -----");

        Ok(())
    }

    fn stop_queues(mut self) -> Result<()> {
        println!("----- STOPPING QUEUES -----");
        for dir in self.directions() {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                ctl::queue_stop("qdmac1000", queue, *dir)?;
                ctl::queue_del("qdmac1000", queue, *dir)?;
            }
        }

        self.needs_clean_up = false;

        Ok(())
    }
}

impl Drop for Bench {
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

fn read_from_queue<const WITH_CTRL_SEQUENCE: bool>(queue: usize, num_packets: usize) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let start = Instant::now();
    for _ in 0..num_packets {
        if WITH_CTRL_SEQUENCE {
            let _received = stream.next_packet_or_ctrl_seq()?;
        } else {
            let _received = stream.next_packet()?;
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
