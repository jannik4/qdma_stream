use qdma_stream::{HostToCardStream, PACKET_SIZE};
use std::{
    io::{self, Read, Seek, Write},
    sync::Arc,
};

pub trait DataSource {
    fn reset(&mut self) -> io::Result<()>;

    fn write_to_stream<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write;

    fn write_to_stream_raw<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write;
}

impl<S> DataSource for S
where
    S: AsRef<[u8]>,
{
    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_to_stream<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        let slice = self.as_ref();
        stream.write_remaining(slice)?;
        Ok(slice.len())
    }

    fn write_to_stream_raw<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        let slice = self.as_ref();
        stream.write_all(slice)?;
        Ok(slice.len())
    }
}

#[derive(Debug, Clone)]
pub struct DataSourceRead<R> {
    reader: R,
    length: usize,
}

impl<R> DataSourceRead<R>
where
    R: Seek,
{
    pub fn new(mut reader: R) -> io::Result<Self> {
        let length = reader.seek(io::SeekFrom::End(0))? as usize;
        reader.seek(io::SeekFrom::Start(0))?;
        Ok(Self { reader, length })
    }
}

impl<R> DataSource for DataSourceRead<R>
where
    R: Read + Seek,
{
    fn reset(&mut self) -> io::Result<()> {
        self.reader.seek(io::SeekFrom::Start(0))?;
        Ok(())
    }

    fn write_to_stream<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        stream.write_complete_stream(&mut self.reader, self.length)?;
        Ok(self.length)
    }

    fn write_to_stream_raw<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        let bytes = io::copy(&mut self.reader, stream)?;
        Ok(bytes as usize)
    }
}

#[derive(Debug, Clone)]
pub struct DataSourceZeroes {
    num_bytes: usize,
    zero_buf: Vec<u8>,
}

impl DataSourceZeroes {
    pub fn new(num_bytes: usize) -> Self {
        Self {
            num_bytes,
            zero_buf: vec![0; PACKET_SIZE],
        }
    }
}

impl DataSource for DataSourceZeroes {
    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_to_stream<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        let mut num_bytes_left = self.num_bytes;
        while num_bytes_left > PACKET_SIZE {
            stream.write_all(&self.zero_buf)?;
            num_bytes_left -= PACKET_SIZE;
        }
        stream.write_remaining(&self.zero_buf[..num_bytes_left])?;
        Ok(self.num_bytes)
    }

    fn write_to_stream_raw<F>(&mut self, stream: &mut HostToCardStream<F>) -> io::Result<usize>
    where
        F: Write,
    {
        let mut num_bytes_left = self.num_bytes;
        while num_bytes_left > PACKET_SIZE {
            stream.write_all(&self.zero_buf)?;
            num_bytes_left -= PACKET_SIZE;
        }
        stream.write_all(&self.zero_buf[..num_bytes_left])?;
        Ok(self.num_bytes)
    }
}

#[derive(Debug, Clone)]
pub struct DataSourceRandom(Arc<[u8]>);

impl DataSourceRandom {
    pub fn new(num_bytes: usize, seed: u64) -> Self {
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

impl AsRef<[u8]> for DataSourceRandom {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
