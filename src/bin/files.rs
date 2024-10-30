use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut f = File::open("/tmp/kk.txt").await?;
    let mut buffer: Vec<u8> = vec![];

    // read up to 10 bytes
    let n = f.read_to_end(&mut buffer).await?;

    println!("The bytes: {} {:?}", n, &buffer[..n]);

    let mut file = File::create("/tmp/kkw.txt").await?;

    // Writes some prefix of the byte string, but not necessarily all of it.
    let to_write = b"some bytes";
    let n = file.write(&to_write[..5]).await?;

    println!("Wrote the first {:?} bytes of 'some bytes'.", &to_write[..n]);
    Ok(())
}
