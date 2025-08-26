#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{DataSinkCountBytes, RunOptions, DEFAULT_DEVICE};

const LOOPBACK_CTRL_MARKER: u8 = 0b11100100;

fn main() -> Result<()> {
    let cmd = Cmd::from_env().context("failed to parse args")?;

    let options = RunOptions {
        device: cmd.device,

        read_len: 0,
        use_raw: true,
        use_unmanaged: cmd.use_unmanaged,
        iterations: 1,

        c2h_queue_start: 0,
        c2h_queue_count: 0,

        h2c_queue_start: 8,
        h2c_queue_count: 1,
    };

    let sink = DataSinkCountBytes::new();
    let source = vec![cmd.loopback_config, LOOPBACK_CTRL_MARKER];
    options.run(source, sink)?;

    Ok(())
}

#[derive(Debug)]
struct Cmd {
    device: String,
    use_unmanaged: bool,
    loopback_config: u8,
}

impl Cmd {
    fn from_env() -> Result<Self> {
        let mut args = pico_args::Arguments::from_env();

        let device = args
            .opt_value_from_str("--device")?
            .unwrap_or_else(|| DEFAULT_DEVICE.to_string());
        let use_unmanaged = args.contains(["-u", "--unmanaged"]);

        let loopback_config_s = args
            .opt_value_from_str(["-l", "--loopback-config"])?
            .unwrap_or_else(|| "00000000".to_string());
        let loopback_config_c = loopback_config_s.chars().collect::<Vec<_>>();
        if loopback_config_c.len() != 8 {
            anyhow::bail!("loopback config must be exactly 8 bits");
        }

        let mut loopback_config = 0u8;
        for (i, c) in loopback_config_c.iter().rev().enumerate() {
            match c {
                '0' => (),
                '1' => loopback_config |= 1 << i,
                _ => anyhow::bail!("loopback config must be a 8 bit pattern"),
            }
        }

        Ok(Self {
            device,
            use_unmanaged,
            loopback_config,
        })
    }
}
