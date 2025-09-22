#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{RunOptions, DEFAULT_DEVICE};

fn main() -> Result<()> {
    let cmd = Cmd::from_env().context("failed to parse args")?;

    // 0xC000_0000
    let mut queue = CommandQueue::new(0x40_0000_0000);

    // queue.write(0x0, &[0; 4096]);
    // queue.read(0x0, 4096);
    // queue.write(0x0, &[1; 2 * 4096]);
    // queue.read(0x0, 2 * 4096);

    queue.write(0x0, &[0; 256]);
    queue.read(0x0, 64);
    queue.write(0x0, &(0..=255).collect::<Vec<_>>());
    queue.read(0x80, 128);

    let options = RunOptions {
        device: cmd.device,

        read_len: queue.read_bytes,
        use_raw: true,
        use_unmanaged: cmd.use_unmanaged,
        iterations: 1,

        c2h_queue_start: 0,
        c2h_queue_count: 1,

        h2c_queue_start: 0,
        h2c_queue_count: 1,
    };

    let source = queue.commands;
    let sink = Vec::new();

    let results = options.run(source, sink)?;

    println!("{:?}", results);

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

struct CommandQueue {
    base_address: u64,
    commands: Vec<u8>,
    read_bytes: usize,
}

impl CommandQueue {
    fn new(base_address: u64) -> Self {
        Self {
            base_address,
            commands: Vec::new(),
            read_bytes: 0,
        }
    }

    fn read(&mut self, mut address: u64, mut len: u64) {
        address += self.base_address;
        self.read_bytes += len as usize;

        while len > 0 {
            let btt = u64::min(len, u16::MAX as u64);

            self.commands
                .extend_from_slice(&u16::to_le_bytes(btt as u16)); // btt
            self.commands.extend_from_slice(&u64::to_le_bytes(address)); // addr
            self.commands.extend_from_slice(&u8::to_le_bytes(0)); // rw flag
            self.commands.extend_from_slice(&u8::to_le_bytes(1)); // wait flag
            self.commands.extend_from_slice(&[0u8; 52]); // padding to 64 bytes

            len -= btt;
            address += btt;
        }
    }

    fn write(&mut self, mut address: u64, data: &[u8]) {
        address += self.base_address;

        for chunk in data.chunks(u16::MAX as usize) {
            let btt = chunk.len() as u64;

            self.commands
                .extend_from_slice(&u16::to_le_bytes(btt as u16)); // btt
            self.commands.extend_from_slice(&u64::to_le_bytes(address)); // addr
            self.commands.extend_from_slice(&u8::to_le_bytes(1)); // rw flag
            self.commands.extend_from_slice(&u8::to_le_bytes(1)); // wait flag
            self.commands.extend_from_slice(&[0u8; 52]); // padding to 64 bytes
            self.commands.extend_from_slice(data); // data

            address += btt;
        }
    }
}
