use anyhow::{Ok, Result};
use qdma_stream::{CardToHostStream, HostToCardStream};
use std::{io::Write, thread};

const LEN: usize = 4096;

fn main() -> Result<()> {
    let queue = 0;

    let data = std::array::from_fn::<_, LEN, _>(|i| ((LEN - 1 - i) % 256) as u8);
    println!("Data: {:?}", &data[0..32]);

    let threads = vec![
        thread::spawn(move || write_to_queue(queue, data)),
        thread::spawn(move || read_from_queue(queue, data)),
    ];

    for t in threads {
        t.join().unwrap()?;
    }

    Ok(())
}

fn write_to_queue(queue: usize, data: [u8; LEN]) -> Result<()> {
    let mut stream = HostToCardStream::new(
        format!("/dev/qdmac1000-ST-{}", queue),
        4096 * 2000,
        4096 * 1000,
        std::time::Duration::from_millis(10),
    )?;

    stream.write_all(&data)?;
    stream.flush()?;

    Ok(())
}

fn read_from_queue(queue: usize, _data: [u8; LEN]) -> Result<()> {
    let mut stream = CardToHostStream::new(format!("/dev/qdmac1000-ST-{}", queue))?;

    let received = stream.next_packet()?;

    println!("Received: {:?}", &received[0..32]);

    Ok(())
}
