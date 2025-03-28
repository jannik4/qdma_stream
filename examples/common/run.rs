use super::{transfer, DataSink, DataSource};
use anyhow::Result;
use qdma_stream::{managed, CardToHostStream, HostToCardStream};
use std::thread;

#[derive(Debug)]
pub struct RunOptions {
    pub device: String,

    pub read_len: usize,
    pub use_raw: bool,
    pub iterations: usize,

    pub c2h_queue_start: usize,
    pub c2h_queue_count: usize,

    pub h2c_queue_start: usize,
    pub h2c_queue_count: usize,
}

impl RunOptions {
    pub fn run<SOURCE, SINK>(self, source: SOURCE, sink: SINK) -> Result<()>
    where
        SOURCE: DataSource + Clone + Send + 'static,
        SINK: DataSink + Clone + Send + 'static,
    {
        println!("----- STARTING QUEUES -----");
        let c2h_queues = (0..self.c2h_queue_count)
            .map(|i| {
                let queue = self.c2h_queue_start + i;
                let c2h_stream = CardToHostStream::new(
                    managed::ManagedCardToHostStreamFile::start(&self.device, queue)?,
                )?;
                Ok((queue, c2h_stream, sink.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        let h2c_queues = (0..self.h2c_queue_count)
            .map(|i| {
                let queue = self.h2c_queue_start + i;
                let h2c_stream = HostToCardStream::new(
                    managed::ManagedHostToCardStreamFile::start(&self.device, queue)?,
                    4096 * 2000,
                    4096 * 1000,
                    std::time::Duration::from_millis(10),
                )?;
                Ok((queue, h2c_stream, source.clone()))
            })
            .collect::<Result<Vec<_>>>()?;

        // Run test
        println!("----- RUNNING TEST -----");
        let mut threads = Vec::new();
        for (queue, c2h_stream, mut sink) in c2h_queues {
            threads.push(thread::spawn(move || {
                transfer::read_from_queue(
                    queue,
                    c2h_stream,
                    &mut sink,
                    self.iterations,
                    self.use_raw.then_some(self.read_len),
                )
            }));
        }
        for (queue, h2c_stream, mut source) in h2c_queues {
            threads.push(thread::spawn(move || {
                transfer::write_to_queue(
                    queue,
                    h2c_stream,
                    &mut source,
                    self.iterations,
                    self.use_raw,
                )
            }));
        }

        // Join threads
        let results = threads.into_iter().map(|t| t.join()).collect::<Vec<_>>();

        // Check results
        for res in results {
            res.unwrap()?;
        }

        Ok(())
    }
}
