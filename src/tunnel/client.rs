use std::time::Duration;

use async_trait::async_trait;
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use tokio_rustls::client::TlsStream;

use log::debug;

use crate::{
    tls::{callbacks::TLSClientCallback, client::ConnectionBuilder},
    tunnel::consts,
};

struct UDSClientConnectionCB {}

#[async_trait]
impl TLSClientCallback for UDSClientConnectionCB {
    async fn process(&self, stream: &mut TcpStream) -> io::Result<()> {
        debug!("Hook for connection established: {:?}", stream);
        // The ssl part if covered by a hadhshake message
        // This handshake has no response, so this is fine
        stream.write_all(consts::HANDSHAKE_V1).await?;

        Ok(())
    }
}

pub async fn connect(
    tunnel_server: &str,
    port: u16,
    verify_ssl: bool,
) -> io::Result<TlsStream<TcpStream>> {
    match timeout(
        Duration::from_secs(8),
        ConnectionBuilder::new(tunnel_server, port)
            .with_connect_callback(UDSClientConnectionCB {})
            .with_verify_ssl(verify_ssl)
            .connect(),
    )
    .await
    {
        Ok(conn) => conn,
        Err(e) => {
            panic!("Error connecting to server: {:?}", e);
        }
    }
}
