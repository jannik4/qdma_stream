#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{RunOptions, DEFAULT_DEVICE};

fn main() -> Result<()> {
    let cmd = Cmd::from_env().context("failed to parse args")?;

    // 0, 1, 6
    // -->
    // 2

    // Write
    for q in [0, 1, 6] {
        let options = RunOptions {
            device: cmd.device.clone(),

            read_len: 0,
            use_raw: true,
            use_unmanaged: cmd.use_unmanaged,
            iterations: 1,

            c2h_queue_start: 0,
            c2h_queue_count: 0,

            h2c_queue_start: q,
            h2c_queue_count: 1,
        };

        let mut source = vec![0u8; 32];
        source[0] = 10;
        let sink = Vec::new();

        let _results = options.run(source, sink)?;
    }

    // Read
    {
        let q = 2;
        let options = RunOptions {
            device: cmd.device,

            read_len: 4096,
            use_raw: true,
            use_unmanaged: cmd.use_unmanaged,
            iterations: 1,

            c2h_queue_start: q,
            c2h_queue_count: 1,

            h2c_queue_start: 0,
            h2c_queue_count: 0,
        };

        let source = Vec::new();
        let sink = Vec::new();

        let results = options.run(source, sink)?;
        println!("{:?}", results);
    }

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
