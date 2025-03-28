use super::*;
use anyhow::Result;
use humansize::{ISizeFormatter, BINARY};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::{
    io::{Read, Write},
    time::Instant,
};

pub fn write_to_queue<F, S>(
    queue: usize,
    mut stream: HostToCardStream<F>,
    data_source: &mut S,
    iterations: usize,
    use_raw: bool,
) -> Result<()>
where
    F: Write,
    S: DataSource,
{
    let start = Instant::now();
    let mut bytes = 0;
    if use_raw {
        for _ in 0..iterations {
            data_source.reset()?;
            bytes += data_source.write_to_stream_raw(&mut stream)?;
        }
    } else {
        for _ in 0..iterations {
            data_source.reset()?;
            bytes += data_source.write_to_stream(&mut stream)?;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();

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

pub fn read_from_queue<F, S>(
    queue: usize,
    mut stream: CardToHostStream<F>,
    data_sink: &mut S,
    iterations: usize,
    use_raw: Option<usize>,
) -> Result<()>
where
    F: Read,
    S: DataSink,
{
    let start = Instant::now();
    let mut bytes = 0;
    match use_raw {
        Some(len) => {
            for _ in 0..iterations {
                data_sink.reset()?;
                bytes += data_sink.read_from_stream_raw(&mut stream, len)?;
            }
        }
        None => {
            for _ in 0..iterations {
                data_sink.reset()?;
                bytes += data_sink.read_from_stream(&mut stream)?;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64();

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
