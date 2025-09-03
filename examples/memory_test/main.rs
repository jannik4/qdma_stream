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

    #[rustfmt::skip]
    let source = vec![
        // Read 64 bytes from address 0x00000000C0000000

        // Bytes to transfer
        64, 0, 
        // Address
        0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00,
        // R/W flag (0/1)
        0,
        // Fill up cmd to 64 bytes
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

        // Write 64 bytes at address 0x00000000C0000000

        // Bytes to transfer
        64, 0, 
        // Address
        0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00,
        // R/W flag (0/1)
        1,
        // Fill up cmd to 64 bytes
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

        // Write payload
         0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,

        // Read 64 bytes from address 0x00000000C0000000

        // Bytes to transfer
        64, 0, 
        // Address
        0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00,
        // R/W flag (0/1)
        0,
        // Fill up cmd to 64 bytes
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
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
