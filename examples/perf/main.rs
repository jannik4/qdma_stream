#[path = "../common/lib.rs"]
mod common;

use anyhow::{Context, Result};
use common::{
    DataSinkCountBytes, DataSourceRandom, DataSourceRead, DataSourceZeroes, RunOptions,
    DEFAULT_DEVICE,
};
use std::{path::PathBuf, sync::Arc};

fn main() -> Result<()> {
    let cmd = Cmd::from_env().context("failed to parse args")?;

    let options = RunOptions {
        device: cmd.device,

        read_len: cmd.input.size()?,
        use_raw: cmd.use_raw,
        use_unmanaged: cmd.use_unmanaged,
        iterations: cmd.iterations,

        c2h_queue_start: cmd.c2h_queue_start,
        c2h_queue_count: cmd.c2h_queue_count,

        h2c_queue_start: cmd.h2c_queue_start,
        h2c_queue_count: cmd.h2c_queue_count,
    };

    macro_rules! run {
        ($options:expr, $source:expr, $output:expr) => {
            match $output {
                Output::CountBytes => {
                    let sink = DataSinkCountBytes::new();
                    $options.run($source, sink)?;
                }
                Output::Buffer => {
                    let sink = Vec::with_capacity($options.read_len);
                    $options.run($source, sink)?;
                }
                Output::File { path } => {
                    let sink = Arc::new(std::fs::File::create(path)?);
                    $options.run($source, sink)?;
                }
            }
        };
    }

    match cmd.input {
        Input::Zeroes { size } => {
            let source = DataSourceZeroes::new(size);
            run!(options, source, cmd.output);
        }
        Input::Random { seed, size } => {
            let source = DataSourceRandom::new(size, seed);
            run!(options, source, cmd.output);
        }
        Input::File { path } => {
            let source = DataSourceRead::new(Arc::new(std::fs::File::open(path)?))?;
            run!(options, source, cmd.output);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Cmd {
    device: String,
    use_raw: bool,
    use_unmanaged: bool,
    c2h_queue_start: usize,
    c2h_queue_count: usize,
    h2c_queue_start: usize,
    h2c_queue_count: usize,
    iterations: usize,
    input: Input,
    output: Output,
}

#[derive(Debug)]
enum Input {
    Zeroes { size: usize },
    Random { seed: u64, size: usize },
    File { path: PathBuf },
}

impl Input {
    fn size(&self) -> Result<usize> {
        match self {
            Self::Zeroes { size } => Ok(*size),
            Self::Random { size, .. } => Ok(*size),
            Self::File { path } => Ok(std::fs::metadata(path)
                .context("failed to get file metadata")?
                .len() as usize),
        }
    }
}

#[derive(Debug)]
enum Output {
    CountBytes,
    Buffer,
    File { path: PathBuf },
}

impl Cmd {
    fn from_env() -> Result<Self> {
        let mut args = pico_args::Arguments::from_env();

        let device = args
            .opt_value_from_str("--device")?
            .unwrap_or_else(|| DEFAULT_DEVICE.to_string());
        let use_raw = args.contains(["-r", "--raw"]);
        let use_unmanaged = args.contains(["-u", "--unmanaged"]);

        // Fallback for both directions
        let queue_start = args
            .opt_value_from_str(["-s", "--queue-start"])?
            .unwrap_or(0);
        let queue_count = args
            .opt_value_from_str(["-c", "--queue-count"])?
            .unwrap_or(0);

        let c2h_queue_start = args
            .opt_value_from_str(["-s", "--c2h-queue-start"])?
            .unwrap_or(queue_start);
        let c2h_queue_count = args
            .opt_value_from_str(["-c", "--c2h-queue-count"])?
            .unwrap_or(queue_count);

        let h2c_queue_start = args
            .opt_value_from_str(["-s", "--h2c-queue-start"])?
            .unwrap_or(queue_start);
        let h2c_queue_count = args
            .opt_value_from_str(["-c", "--h2c-queue-count"])?
            .unwrap_or(queue_count);

        let iterations = args
            .opt_value_from_str(["-i", "--iterations"])?
            .unwrap_or(1);

        let input = match args.opt_value_from_str(["-f", "--file"])? {
            Some(file) => Input::File { path: file },
            None => {
                let size = args.opt_value_from_str("--size")?.unwrap_or(4096);
                let seed = args.opt_value_from_str("--seed")?;

                match seed {
                    Some(seed) => Input::Random { seed, size },
                    None => Input::Zeroes { size },
                }
            }
        };
        let output = if args.contains("--output-buffer") {
            Output::Buffer
        } else if let Some(file) = args.opt_value_from_str("--output-file")? {
            if queue_count != 1 {
                anyhow::bail!("output file can only be used with a single queue");
            }
            Output::File { path: file }
        } else {
            Output::CountBytes
        };

        Ok(Self {
            device,
            use_raw,
            use_unmanaged,
            c2h_queue_start,
            c2h_queue_count,
            h2c_queue_start,
            h2c_queue_count,
            iterations,
            input,
            output,
        })
    }
}
