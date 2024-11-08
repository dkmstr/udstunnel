use log;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{timeout, Duration},
};
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        version::{TLS12, TLS13},
        ServerConfig,
    },
    TlsAcceptor,
};

use super::super::{config, tls};
use super::{consts, error};

pub async fn launch(config: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let certs = CertificateDer::from_pem_file(config.ssl_certificate).unwrap();
    let private_key: PrivateKeyDer<'_> =
        PrivateKeyDer::from_pem_file(config.ssl_certificate_key).unwrap();

    let protocol_versions: Vec<&rustls::SupportedProtocolVersion> =
        match config.ssl_min_tls_version.as_str() {
            "1.2" => vec![&TLS12],
            "1.3" => vec![&TLS13],
            _ => vec![&TLS12, &TLS13],
        };

    let server_ssl_config = ServerConfig::builder_with_provider(Arc::new(
        tls::crypto_provider::provider(&config.ssl_ciphers),
    ))
    .with_protocol_versions(&protocol_versions)
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certs], private_key)?;

    log::debug!(
        "cipher_suites: {:?}",
        server_ssl_config.crypto_provider().cipher_suites
    );

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

    log::info!("Tunnel server running on {}", address);

    let listener = TcpListener::bind(address).await?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();

        let task = tokio::spawn(async move {
            log::debug!("CONNECTION from {:?}", stream.peer_addr().unwrap());

            let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

            // 1.- Read the handshake (with timeout)
            let handshake =
                match timeout(consts::HANDSHAKE_TIMEOUT, stream.read_exact(&mut buf)).await {
                    Ok(handshake) => handshake,
                    Err(e) => Err(e.into()),
                };

            if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                error::log_handshake_error(&stream, &buf, false).await;
                stream.shutdown().await.unwrap_or_else(|e| {
                    log::warn!("Could not shutdown stream: {:?}", e);
                });
                return;
            }
            let mut stream = acceptor.accept(stream).await.unwrap();

            // Now, we expect the command

            stream.shutdown().await.unwrap();
        });

        task.await?;
    }
}
