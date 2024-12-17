use anyhow::{anyhow, Result};
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
        match self.file.read(buf)? {
            0 => Ok(None),
            4096 => {
                if buf.starts_with(&CTRL_SEQ) {
                    match self.file.read(buf)? {
                        4096 => {
                            if buf.starts_with(&CTRL_SEQ) {
                                Ok(Some(()))
                            } else {
                                Ok(None)
                            }
                        }
                        count => Err(anyhow!("read {} bytes, expect 4096 bytes", count)),
                    }
                } else {
                    Ok(Some(()))
                }
            }
            count => Err(anyhow!("read {} bytes, expect 4096 bytes", count)),
        }
    }
}

const CTRL_SEQ: [u8; 4] = [0x4A, 0x37, 0xF1, 0x5C];
