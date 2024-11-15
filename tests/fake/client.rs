use std::time::Duration;
use tokio::{net::TcpStream, time::timeout};
use tokio_rustls::client::TlsStream;
use udstunnel::tunnel::client;

#[allow(dead_code)]
pub async fn open_client_no_handshake(port: u16) -> TcpStream {
    // Let try to connect to the server
    // Check it's listening...
    match timeout(
        Duration::from_millis(200),
        TcpStream::connect(format!("{}:{}", "localhost", port)),
    )
    .await
    {
        Ok(conn) => conn,
        Err(e) => {
            panic!("Error connecting to server: {:?}", e);
        }
    }
    .unwrap()
}

#[allow(dead_code)]
pub async fn open_client_with_handshake(port: u16) -> TlsStream<TcpStream> {
    client::connect("localhost", port, false).await.unwrap()
}
