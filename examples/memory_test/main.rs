#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{RunOptions, DEFAULT_DEVICE};

fn main() -> Result<()> {
    let cmd = Cmd::from_env().context("failed to parse args")?;

    let options = RunOptions {
        device: cmd.device,

        read_len: 128,
        use_raw: true,
        use_unmanaged: cmd.use_unmanaged,
        iterations: 1,

        c2h_queue_start: 0,
        c2h_queue_count: 1,

        h2c_queue_start: 0,
        h2c_queue_count: 1,
    };

    let mut cmds = CommandQueue::new();
    cmds.write(0x0000_0000_C000_0000, &[0; 64]);
    cmds.read(0x0000_0000_C000_0000, 64);
    cmds.write(0x0000_0000_C000_0000, &(1..=64).collect::<Vec<_>>());
    cmds.read(0x0000_0000_C000_0000, 64);

    let source = cmds.0;
    let sink = Vec::new();

    let results = options.run(source, sink)?;

    dbg!(results);

    Ok(())
}

#[derive(Debug)]
struct Cmd {
    device: String,
    use_unmanaged: bool,
}

impl Cmd {
    fn from_env() -> Result<Self> {
        let mut args = pico_args::Arguments::from_env();

        let device = args
            .opt_value_from_str("--device")?
            .unwrap_or_else(|| DEFAULT_DEVICE.to_string());
        let use_unmanaged = args.contains(["-u", "--unmanaged"]);

        Ok(Self {
            device,
            use_unmanaged,
        })
    }
}

struct CommandQueue(Vec<u8>);

impl CommandQueue {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn read(&mut self, mut address: u64, mut len: u64) {
        while len > 0 {
            let btt = u64::min(len, u16::MAX as u64);

            self.0.extend_from_slice(&u16::to_le_bytes(btt as u16)); // btt
            self.0.extend_from_slice(&u64::to_le_bytes(address)); // addr
            self.0.extend_from_slice(&u8::to_le_bytes(0)); // rw flag
            self.0.extend_from_slice(&u8::to_le_bytes(0)); // wait flag
            self.0.extend_from_slice(&[0u8; 52]); // padding to 64 bytes

            len -= btt;
            address += btt;
        }
    }

    fn write(&mut self, mut address: u64, data: &[u8]) {
        for chunk in data.chunks(u16::MAX as usize) {
            let btt = chunk.len() as u64;

            self.0.extend_from_slice(&u16::to_le_bytes(btt as u16)); // btt
            self.0.extend_from_slice(&u64::to_le_bytes(address)); // addr
            self.0.extend_from_slice(&u8::to_le_bytes(1)); // rw flag
            self.0.extend_from_slice(&u8::to_le_bytes(0)); // wait flag
            self.0.extend_from_slice(&[0u8; 52]); // padding to 64 bytes
            self.0.extend_from_slice(data); // data

            address += btt;
        }
    }
}
