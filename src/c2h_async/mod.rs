mod buf;

use self::buf::Buf;
use anyhow::Result;
use monoio::fs;

pub struct CardToHostStreamAsync {
    file: fs::File,
    buf: Option<Buf>,
    pos: u64,
}

impl CardToHostStreamAsync {
    pub const PACKET_SIZE: usize = 4096;
    const ALIGN: usize = 4096;

    pub async fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(path.as_ref())
            .await?;

        let buf = Some(Buf::new()?);

        Ok(Self { file, buf, pos: 0 })
    }

    pub async fn next_packet(&mut self) -> Result<&[u8]> {
        self.read().await?;
        Ok(self.slice())
    }

    pub async fn next_packet_or_ctrl_seq(&mut self) -> Result<Option<&[u8]>> {
        self.read().await?;
        if self.slice().starts_with(&CTRL_SEQ) {
            self.read().await?;
            if self.slice().starts_with(&CTRL_SEQ) {
                Ok(Some(self.slice()))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(self.slice()))
        }
    }

    async fn read(&mut self) -> Result<()> {
        let buf = self.buf.take().unwrap();
        let (res, buf) = self.file.read_exact_at(buf, self.pos).await;
        self.buf = Some(buf);
        res?;
        self.pos += Self::PACKET_SIZE as u64;
        Ok(())
    }

    fn slice(&self) -> &[u8] {
        self.buf.as_ref().unwrap().as_slice()
    }
}

const CTRL_SEQ: [u8; 4] = [0x4A, 0x37, 0xF1, 0x5C];
