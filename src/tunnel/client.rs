use async_trait::async_trait;
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

use tokio_rustls::client::TlsStream;


use env_logger::Env;
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

pub async fn connect(tunnel_server: &str, port: u16, verify_ssl: bool) -> io::Result<TlsStream<TcpStream>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    return ConnectionBuilder::new(tunnel_server, port)
        .with_connect_callback(UDSClientConnectionCB {})
        .with_verify_ssl(verify_ssl)
        .connect()
        .await;

}
