use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{ctl, CardToHostStream, HostToCardStream};
use std::{error::Error, io::Write, str::FromStr, sync::Arc, thread, time::Instant};

fn main() -> Result<()> {
    let queue_start = parse_arg(1, 0)?;
    let queue_count = parse_arg(2, 4)?;
    let data_len = parse_arg(3, 1024 * 1024)?;
    let data_iterations = parse_arg(4, 1)?;
    let seed = parse_arg(5, 0)?;

    Test::new(queue_start, queue_count, data_len, data_iterations, seed).run()?;

    Ok(())
}

fn parse_arg<T: FromStr<Err: Error + Send + Sync + 'static>>(n: usize, default: T) -> Result<T> {
    Ok(std::env::args()
        .nth(n)
        .as_deref()
        .map(str::parse)
        .transpose()?
        .unwrap_or(default))
}

struct Test {
    queue_start: usize,
    queue_count: usize,
    data_len: usize,
    data_iterations: usize,
    seed: Option<u64>,
    needs_clean_up: bool,
}

impl Test {
    fn new(
        queue_start: usize,
        queue_count: usize,
        data_len: usize,
        data_iterations: usize,
        seed: u64,
    ) -> Self {
        Self {
            queue_start,
            queue_count,
            data_len,
            data_iterations,
            seed: (seed != 0).then_some(seed),
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
        println!("Data length: {}", self.data_len);
        println!("Data iterations: {}", self.data_iterations);
        println!("Seed: {:?}", self.seed);

        println!("----- STARTING QUEUES -----");
        for dir in [ctl::QueueDir::C2h, ctl::QueueDir::H2c] {
            for queue in self.queue_start..self.queue_start + self.queue_count {
                ctl::queue_add("qdmac1000", queue, dir)?;
                ctl::queue_start("qdmac1000", queue, dir)?;
            }
        }

        // Prepare queues and buffers
        println!("----- PREPARING QUEUES AND DATA -----");
        let (data, receive_buffer) = match self.seed {
            Some(seed) => {
                let data = TestData::random_data(self.data_len, seed);
                let receive_buffer = ReceiveBuffer::Vec(Vec::with_capacity(data.len()));
                (data, receive_buffer)
            }
            None => {
                let data = TestData::zeroes(self.data_len);
                let receive_buffer = ReceiveBuffer::CountBytes { count: 0 };
                (data, receive_buffer)
            }
        };
        let queues = (0..self.queue_count)
            .map(|i| {
                let queue = self.queue_start + i;
                let h2c_stream = HostToCardStream::new(
                    format!("/dev/qdmac1000-ST-{}", queue),
                    4096 * 2000,
                    4096 * 1000,
                    std::time::Duration::from_millis(10),
                )?;
                let c2h_stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;
                Ok((
                    queue,
                    h2c_stream,
                    c2h_stream,
                    data.clone(),
                    data.clone(),
                    receive_buffer.clone(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        // Run test
        println!("----- RUNNING TEST -----");
        let mut threads = Vec::new();
        for (queue, h2c_stream, c2h_stream, data_write, data_read, receive_buffer) in queues {
            threads.push(thread::spawn(move || {
                write_to_queue(queue, h2c_stream, data_write, self.data_iterations)
            }));
            threads.push(thread::spawn(move || {
                read_from_queue(
                    queue,
                    c2h_stream,
                    data_read,
                    receive_buffer,
                    self.data_iterations,
                )
            }));
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

fn write_to_queue(
    queue: usize,
    mut stream: HostToCardStream,
    test_data: TestData,
    data_iterations: usize,
) -> Result<()> {
    let start = Instant::now();
    match &test_data {
        TestData::Data(data) => {
            for _ in 0..data_iterations {
                stream.write_remaining(data)?
            }
        }
        TestData::Zeroes {
            num_bytes,
            zero_buf,
        } => {
            for _ in 0..data_iterations {
                let mut num_bytes_left = *num_bytes;
                while num_bytes_left > 4096 {
                    stream.write_all(zero_buf)?;
                    num_bytes_left -= 4096;
                }
                stream.write_remaining(&zero_buf[..num_bytes_left])?;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = data_iterations * test_data.len();
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

fn read_from_queue(
    queue: usize,
    mut stream: CardToHostStream,
    test_data: TestData,
    mut receive_buffer: ReceiveBuffer,
    data_iterations: usize,
) -> Result<()> {
    let start = Instant::now();
    for _ in 0..data_iterations {
        receive_buffer.clear();
        stream.read_complete_protocol(&mut receive_buffer)?;
    }
    let elapsed = start.elapsed().as_secs_f64();

    let bytes = data_iterations * test_data.len();
    let speed = bytes as f64 / elapsed;
    println!(
        "queue({}): read {} bytes in {:.6} seconds @ {}/s",
        queue,
        ISizeFormatter::new(bytes, BINARY),
        elapsed,
        ISizeFormatter::new(speed, BINARY),
    );

    // Check result of last iteration
    match &receive_buffer {
        ReceiveBuffer::Vec(received) => match &test_data {
            TestData::Data(data) => {
                if received != &**data {
                    return Err(anyhow::anyhow!("packet mismatch"));
                }
            }
            TestData::Zeroes { num_bytes, .. } => {
                if received.len() != *num_bytes || received.iter().any(|b| *b != 0) {
                    return Err(anyhow::anyhow!("packet mismatch"));
                }
            }
        },
        ReceiveBuffer::CountBytes { count } => {
            if *count != test_data.len() {
                return Err(anyhow::anyhow!("packet mismatch"));
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
enum TestData {
    Data(Arc<[u8]>),
    Zeroes { num_bytes: usize, zero_buf: Vec<u8> },
}

impl TestData {
    fn random_data(num_bytes: usize, seed: u64) -> Self {
        let mut state = u64::max(1, seed);
        Self::Data(
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

    fn zeroes(num_bytes: usize) -> Self {
        Self::Zeroes {
            num_bytes,
            zero_buf: vec![0; 4096],
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Data(data) => data.len(),
            Self::Zeroes { num_bytes, .. } => *num_bytes,
        }
    }
}

#[derive(Debug, Clone)]
enum ReceiveBuffer {
    Vec(Vec<u8>),
    CountBytes { count: usize },
}

impl ReceiveBuffer {
    fn clear(&mut self) {
        match self {
            Self::Vec(v) => v.clear(),
            Self::CountBytes { count } => *count = 0,
        }
    }
}

impl Write for ReceiveBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Vec(v) => {
                v.clear();
                v.write(buf)
            }
            Self::CountBytes { count } => {
                *count += buf.len();
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
