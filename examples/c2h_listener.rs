use anyhow::Result;
use std::{fs, io::Read, thread};

fn main() -> Result<()> {
    let queue = std::env::args()
        .nth(1)
        .unwrap_or("0".to_string())
        .parse::<u32>()?;

    let mut file = fs::File::open(format!("/dev/qdmac1000-ST-{}", queue))?;

    let mut buf = vec![0; 4096 * 1000];

    loop {
        match file.read(&mut buf) {
            Ok(count) => println!("queue({}): read {} bytes", queue, count),
            Err(e) => eprintln!("queue({}): {}", queue, e),
        }

        thread::sleep(std::time::Duration::from_secs(1));
    }
}
