use async_trait::async_trait;
use tokio::{io, net::TcpStream};

#[async_trait]
pub trait TLSClientCallback: Send + Sync {
    async fn process(&self, stream: &mut TcpStream) -> io::Result<()>;
}
