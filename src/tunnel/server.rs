use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};
use log::{error, debug};

use super::consts;

const CERT_BYTES: &[u8] = include_bytes!("../../cert.pem");
const KEY_BYTES: &[u8] = include_bytes!("../../key.pem");

pub async fn launch() -> Result<(), Box<dyn std::error::Error>> {
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
        let (mut stream, _) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();

        let task = tokio::spawn(async move {
            // Read HANDSHAKE first
            let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

            let handshake = stream.read_exact(&mut buf).await;

            if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                error!("Error reading handshake");
                return;
            }
            debug!("Handshake: {:?}", buf);
        
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
