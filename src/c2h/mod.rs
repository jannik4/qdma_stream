use anyhow::Result;
use std::{fs, io::Read};

pub struct CardToHostStream {
    file: fs::File,
}

impl CardToHostStream {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(path.as_ref())?;

        Ok(Self { file })
    }

    pub fn next_packet(&mut self, buf: &mut [u8; 4096]) -> Result<Option<()>> {
        self.file.read_exact(buf)?;
        if buf.starts_with(&CTRL_SEQ) {
            self.file.read_exact(buf)?;
            if buf.starts_with(&CTRL_SEQ) {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(()))
        }
    }
}

const CTRL_SEQ: [u8; 4] = [0x4A, 0x37, 0xF1, 0x5C];
