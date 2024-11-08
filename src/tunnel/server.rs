use log;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
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

use rustls::crypto::aws_lc_rs;

// fn provider(list_of_ciphers: &String) -> rustls::crypto::CryptoProvider {
//     debug!("ALL: {:?}", rustls::crypto::aws_lc_rs::ALL_CIPHER_SUITES.to_vec());
//     rustls::crypto::CryptoProvider {
//         cipher_suites: rustls::crypto::aws_lc_rs::ALL_CIPHER_SUITES.to_vec(),
//         ..aws_lc_rs::default_provider()
//     }
// }
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

    let server_ssl_config = ServerConfig::builder_with_provider(Arc::new(tls::crypto_provider::provider(&config.ssl_ciphers)))
        .with_protocol_versions(&protocol_versions).unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![certs], private_key)?;

    log::debug!("cipher_suites: {:?}", server_ssl_config.crypto_provider().cipher_suites);

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

            // Read HANDSHAKE first
            let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

            let handshake = stream.read_exact(&mut buf).await;

            if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                error::log_handshake_error(&stream, &buf);
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
