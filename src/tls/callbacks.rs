use async_trait::async_trait;
use tokio::{io, net::TcpStream};

use tokio_rustls::server::TlsStream;

#[async_trait]
pub trait TLSConnectCallback: Send + Sync {
    async fn process(&self, stream: &mut TcpStream) -> io::Result<()>;
}
