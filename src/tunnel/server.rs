use log::{debug, error, info};
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

use super::super::config;
use super::consts;

pub async fn launch(config: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    // Configure TLS
    let certs = CertificateDer::from_pem_file(config.ssl_certificate).unwrap();
    let private_key: PrivateKeyDer<'_> = PrivateKeyDer::from_pem_file(config.ssl_certificate_key).unwrap();

    let server_ssl_config: ServerConfig = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certs], private_key)?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(server_ssl_config));

    let address = if config.ipv6 {
        // If listen address already has brackets, don't add them
        let listen_address = if config.listen_address.starts_with('[') {
            config.listen_address.clone()
        } else {
            format!("[{}]", config.listen_address)
        };
        format!("{}:{}", listen_address, config.listen_port)
    } else {
        format!("{}:{}", config.listen_address, config.listen_port)
    };

    info!("Servidor TLS corriendo en {}", address);

    let listener = TcpListener::bind(address).await?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();

        let task = tokio::spawn(async move {
            // Read HANDSHAKE first
            let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

            let handshake = stream.read_exact(&mut buf).await;

            if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                error!("Error reading handshake: {:?}", handshake);
                return;
            }
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
