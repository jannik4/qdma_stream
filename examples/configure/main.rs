#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{DataSinkCountBytes, RunOptions, DEFAULT_DEVICE};

const LOOPBACK_CTRL_MARKER: u8 = 0b11100100;
const MEMORY_CTRL_MARKER: u8 = 0b11101000;

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

    match cmd.config {
        Config::Loopback(loopback_config) => {
            let source = vec![loopback_config, LOOPBACK_CTRL_MARKER];
            options.run(source, sink)?;
        }
        Config::Memory(memory_config) => {
            let source = vec![memory_config as u8, MEMORY_CTRL_MARKER];
            options.run(source, sink)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Cmd {
    device: String,
    use_unmanaged: bool,
    config: Config,
}

#[derive(Debug)]
enum Config {
    Loopback(u8),
    Memory(bool),
}

impl Cmd {
    fn from_env() -> Result<Self> {
        let mut args = pico_args::Arguments::from_env();

        let device = args
            .opt_value_from_str("--device")?
            .unwrap_or_else(|| DEFAULT_DEVICE.to_string());
        let use_unmanaged = args.contains(["-u", "--unmanaged"]);

        let loopback_config = loopback_config(&mut args)?;
        let memory_config = memory_config(&mut args)?;

        let config = match (loopback_config, memory_config) {
            (Some(loopback_config), None) => Config::Loopback(loopback_config),
            (None, Some(memory_config)) => Config::Memory(memory_config),
            _ => {
                anyhow::bail!(
                    "exactly one of --loopback-config or --memory-config must be specified"
                );
            }
        };

        Ok(Self {
            device,
            use_unmanaged,
            config,
        })
    }
}

fn loopback_config(args: &mut pico_args::Arguments) -> Result<Option<u8>> {
    let Some(loopback_config_s) =
        args.opt_value_from_str::<_, String>(["-l", "--loopback-config"])?
    else {
        return Ok(None);
    };
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

    Ok(Some(loopback_config))
}

fn memory_config(args: &mut pico_args::Arguments) -> Result<Option<bool>> {
    let Some(memory_config) = args.opt_value_from_str::<_, String>(["-m", "--memory-config"])?
    else {
        return Ok(None);
    };
    let memory_config = match memory_config.as_str() {
        "0" => false,
        "1" => true,
        _ => anyhow::bail!("memory config must be '0' or '1'"),
    };
    Ok(Some(memory_config))
}
