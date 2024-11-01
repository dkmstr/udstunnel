use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
};

use std::cmp::min;

use env_logger::Env;
use log::debug;

use iotest::tls::connect::ConnectionBuilder;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let mut tls_stream = ConnectionBuilder::new("db.dkmon.com", 443)
        .with_callback(async {
            debug!("Connection established. Doing SSL");
        })
        .with_verify(false)
        .connect()
        .await
        .unwrap();

    tls_stream
        .write_all(
            concat!(
                "GET / HTTP/1.1\r\n",
                "Host: www.rust-lang.org\r\n",
                "Connection: close\r\n",
                "Accept-Encoding: identity\r\n",
                "\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let ciphersuite = tls_stream.get_ref().1.negotiated_cipher_suite().unwrap();
    debug!("Current ciphersuite: {:?}", ciphersuite.suite());
    let mut plaintext = Vec::new();
    tls_stream.read_to_end(&mut plaintext).await.unwrap();
    debug!("Read {} bytes", plaintext.len());
    // First 512 bytes if available of the plaintext.
    let plaintext = &plaintext[..min(512, plaintext.len())];
    debug!("Plaintext: {:?}", String::from_utf8_lossy(plaintext));

    Ok(())
}
