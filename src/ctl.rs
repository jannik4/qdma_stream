use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueDir {
    C2h,
    H2c,
}

impl QueueDir {
    pub fn as_str(&self) -> &str {
        match self {
            QueueDir::C2h => "c2h",
            QueueDir::H2c => "h2c",
        }
    }
}

pub fn queue_add(device: &str, queue: usize, dir: QueueDir) -> Result<()> {
    execute_dma_ctl(&[
        device,
        "q",
        "add",
        "idx",
        &queue.to_string(),
        "mode",
        "st",
        "dir",
        dir.as_str(),
    ])
}

pub fn queue_start(device: &str, queue: usize, dir: QueueDir) -> Result<()> {
    match dir {
        QueueDir::C2h => execute_dma_ctl(&[
            device,
            "q",
            "start",
            "idx",
            &queue.to_string(),
            "dir",
            dir.as_str(),
        ]),
        QueueDir::H2c => execute_dma_ctl(&[
            device,
            "q",
            "start",
            "idx",
            &queue.to_string(),
            "dir",
            dir.as_str(),
            "fetch_credit",
            "h2c",
        ]),
    }
}

pub fn queue_stop(device: &str, queue: usize, dir: QueueDir) -> Result<()> {
    execute_dma_ctl(&[
        device,
        "q",
        "stop",
        "idx",
        &queue.to_string(),
        "dir",
        dir.as_str(),
    ])
}

pub fn queue_del(device: &str, queue: usize, dir: QueueDir) -> Result<()> {
    execute_dma_ctl(&[
        device,
        "q",
        "del",
        "idx",
        &queue.to_string(),
        "dir",
        dir.as_str(),
    ])
}

fn execute_dma_ctl(args: &[&str]) -> Result<()> {
    let output = Command::new("dma-ctl").args(args).output()?;

    if !output.status.success() {
        anyhow::bail!(
            "failed to execute dma-ctl: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
