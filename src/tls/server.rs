use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};

const CERT_BYTES: &[u8] = include_bytes!("../cert.pem");
const KEY_BYTES: &[u8] = include_bytes!("../key.pem");

struct ServerBuilder {
    certificate: Vec<u8>,
    private_key: Vec<u8>,
    server: String,
    port: u16,
}

impl ServerBuilder {
    pub fn new(certificate: &[u8], privte_key: &[u8]) -> Self {
        ServerBuilder {
            certificate: Vec::from(certificate),
            private_key: Vec::from(privte_key),
            server: String::from("[::]"),
            port: 4443,
        }
    }

    pub fn with_server(mut self, server: &str) -> Self {
        self.server = String::from(server);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn setup(self) -> Result<(TcpListener, TlsAcceptor), Box<dyn std::error::Error>> {
        let certs = CertificateDer::from_pem_slice(&self.certificate).unwrap();
        let private_key = PrivateKeyDer::from_pem_slice(&self.private_key).unwrap();

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certs], private_key)?;

        let tls_acceptor = TlsAcceptor::from(Arc::new(config));

        let listener = TcpListener::bind(format!("{}:{}", self.server, self.port)).await?;

        Ok((listener, tls_acceptor))
    }

}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure TLS
    let certs = CertificateDer::from_pem_slice(CERT_BYTES).unwrap();
    let private_key = PrivateKeyDer::from_pem_slice(KEY_BYTES).unwrap();

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certs], private_key)?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(config));

    // Inicia el servidor TCP
    let listener = TcpListener::bind("[::]:4443").await?;
    println!("Servidor TLS corriendo en 0.0.0.0:4443");
    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();

        let task = tokio::spawn(async move {
            let mut stream = acceptor.accept(stream).await.unwrap();

            // Send a message to the client
            if stream.write_all(b"Hi from TLS!\n").await.is_err() {
                return;
            }
            stream.shutdown().await.unwrap();
        });

        task.await?;
    }

}
