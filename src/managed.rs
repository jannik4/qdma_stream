use crate::ctl;
use anyhow::Result;
use std::{
    fs,
    io::{Read, Write},
};

pub struct ManagedCardToHostStreamFile {
    device: String,
    queue: usize,
    file: fs::File,
}

impl ManagedCardToHostStreamFile {
    pub fn start(device: &str, queue: usize) -> Result<Self> {
        ctl::queue_add(device, queue, ctl::QueueDir::C2h)?;
        ctl::queue_start(device, queue, ctl::QueueDir::C2h)?;

        let file = fs::OpenOptions::new()
            .read(true)
            .open(format!("/dev/{}-ST-{}", device, queue))?;

        Ok(Self {
            device: device.to_string(),
            queue,
            file,
        })
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    pub fn queue(&self) -> usize {
        self.queue
    }

    pub fn stop(self) -> Result<()> {
        self.stop_impl()
    }

    fn stop_impl(&self) -> Result<()> {
        ctl::queue_stop(&self.device, self.queue, ctl::QueueDir::C2h)?;
        ctl::queue_del(&self.device, self.queue, ctl::QueueDir::C2h)?;
        Ok(())
    }
}

impl Read for ManagedCardToHostStreamFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.file.read_vectored(bufs)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.file.read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.file.read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.file.read_exact(buf)
    }
}

impl Drop for ManagedCardToHostStreamFile {
    fn drop(&mut self) {
        if let Err(err) = self.stop_impl() {
            eprintln!("Failed to stop queue: {:?}", err);
        }
    }
}

pub struct ManagedHostToCardStreamFile {
    device: String,
    queue: usize,
    file: fs::File,
}

impl ManagedHostToCardStreamFile {
    pub fn start(device: &str, queue: usize) -> Result<Self> {
        ctl::queue_add(device, queue, ctl::QueueDir::H2c)?;
        ctl::queue_start(device, queue, ctl::QueueDir::H2c)?;

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/{}-ST-{}", device, queue))?;

        Ok(Self {
            device: device.to_string(),
            queue,
            file,
        })
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    pub fn queue(&self) -> usize {
        self.queue
    }

    pub fn stop(self) -> Result<()> {
        self.stop_impl()
    }

    fn stop_impl(&self) -> Result<()> {
        ctl::queue_stop(&self.device, self.queue, ctl::QueueDir::H2c)?;
        ctl::queue_del(&self.device, self.queue, ctl::QueueDir::H2c)?;
        Ok(())
    }
}

impl Write for ManagedHostToCardStreamFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.file.write_vectored(bufs)
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.file.write_all(buf)
    }
}

impl Drop for ManagedHostToCardStreamFile {
    fn drop(&mut self) {
        if let Err(err) = self.stop_impl() {
            eprintln!("Failed to stop queue: {:?}", err);
        }
    }
}
