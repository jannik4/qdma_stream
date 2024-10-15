use anyhow::Result;
use qdma_stream::HostToCardStream;
use std::io::Write;

fn main() -> Result<()> {
    let mut stream = HostToCardStream::new(
        "/dev/qdmac1000-ST-0",
        4096,
        4096,
        std::time::Duration::from_secs(1),
    )?;

    let buf = vec![0; 4096];

    for _ in 0..1000 {
        stream.write_all(&buf)?;
    }

    Ok(())
}
