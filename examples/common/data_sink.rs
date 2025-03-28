use qdma_stream::{CardToHostStream, PACKET_SIZE};
use std::{
    fs,
    io::{self, Read, Seek, Write},
    sync::Arc,
};

pub trait DataSink {
    fn reset(&mut self) -> io::Result<()>;

    fn read_from_stream<F>(&mut self, stream: &mut CardToHostStream<F>) -> io::Result<usize>
    where
        F: Read;

    fn read_from_stream_raw<F>(
        &mut self,
        stream: &mut CardToHostStream<F>,
        len: usize,
    ) -> io::Result<usize>
    where
        F: Read;
}

impl<W> DataSink for W
where
    W: Write + Reset,
{
    fn reset(&mut self) -> io::Result<()> {
        self.reset_impl()?;
        Ok(())
    }

    fn read_from_stream<F>(&mut self, stream: &mut CardToHostStream<F>) -> io::Result<usize>
    where
        F: Read,
    {
        stream.read_complete_stream(self)
    }

    fn read_from_stream_raw<F>(
        &mut self,
        stream: &mut CardToHostStream<F>,
        len: usize,
    ) -> io::Result<usize>
    where
        F: Read,
    {
        let mut num_bytes_left = len;
        while num_bytes_left > PACKET_SIZE {
            self.write_all(stream.next_raw_packet()?)?;
            num_bytes_left -= PACKET_SIZE;
        }
        if num_bytes_left != 0 {
            self.write_all(stream.next_raw_packet_with_len(num_bytes_left)?)?;
        }
        Ok(len)
    }
}

#[derive(Debug, Clone)]
pub struct DataSinkCountBytes {
    count: usize,
}

impl DataSinkCountBytes {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    // pub fn count(&self) -> usize {
    //     self.count
    // }
}

impl DataSink for DataSinkCountBytes {
    fn reset(&mut self) -> io::Result<()> {
        self.count = 0;
        Ok(())
    }

    fn read_from_stream<F>(&mut self, stream: &mut CardToHostStream<F>) -> io::Result<usize>
    where
        F: Read,
    {
        let mut bytes = 0;
        loop {
            let (is_last, packet) = stream.next_stream_packet()?;
            self.count += packet.len();
            bytes += packet.len();
            if is_last {
                break Ok(bytes);
            }
        }
    }

    fn read_from_stream_raw<F>(
        &mut self,
        stream: &mut CardToHostStream<F>,
        len: usize,
    ) -> io::Result<usize>
    where
        F: Read,
    {
        let mut num_bytes_left = len;
        while num_bytes_left > PACKET_SIZE {
            stream.next_raw_packet()?;
            self.count += PACKET_SIZE;
            num_bytes_left -= PACKET_SIZE;
        }
        if num_bytes_left != 0 {
            stream.next_raw_packet_with_len(num_bytes_left)?;
            self.count += num_bytes_left;
        }
        Ok(len)
    }
}

// ----------------------------------------------------------------------------

trait Reset {
    fn reset_impl(&mut self) -> io::Result<()>;
}

impl Reset for Arc<fs::File> {
    fn reset_impl(&mut self) -> io::Result<()> {
        self.seek(io::SeekFrom::Start(0))?;
        Ok(())
    }
}

impl Reset for fs::File {
    fn reset_impl(&mut self) -> io::Result<()> {
        self.seek(io::SeekFrom::Start(0))?;
        Ok(())
    }
}

impl Reset for Vec<u8> {
    fn reset_impl(&mut self) -> io::Result<()> {
        self.clear();
        Ok(())
    }
}
