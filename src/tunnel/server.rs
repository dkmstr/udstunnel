use log;
use std::{f32::consts::E, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        version::{TLS12, TLS13},
        ServerConfig,
    },
    TlsAcceptor,
};
use uuid;

use crate::tunnel::types;

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

    let server_tls_config = ServerConfig::builder_with_provider(Arc::new(
        tls::crypto_provider::provider(&config.ssl_ciphers),
    ))
    .with_protocol_versions(&protocol_versions)
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certs], private_key)?;

    log::debug!(
        "cipher_suites: {:?}",
        server_tls_config.crypto_provider().cipher_suites
    );

    let tls_acceptor = TlsAcceptor::from(Arc::new(server_tls_config));

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
        let connection_id = uuid::Uuid::new_v4().to_string()[..13].to_string();
        let from: String = stream.peer_addr().unwrap().to_string();

        let task = tokio::spawn(async move {
            log::info!("CONNECTION ({connection_id}) from {from}");

            let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

            // 1.- Read the handshake (with timeout)
            let handshake =
                match timeout(consts::HANDSHAKE_TIMEOUT, stream.read_exact(&mut buf)).await {
                    Ok(handshake) => handshake,
                    Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                };

            // If no valid, even if timeout, close the connection and log the error
            if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                // If it's not a timeout, log the error
                log_error(handshake.err(), &buf, &connection_id, &from, "HANDSHAKE").await;
                stream.shutdown().await.unwrap_or_else(|e| {
                    log::warn!("Could not shutdown stream: {:?}", e);
                });
                return;
            }

            log::debug!("HANDSHAKE ({connection_id}) from {from}");

            // 2.- Upgrade the connection to TLS
            let mut stream = acceptor.accept(stream).await.unwrap();

            // Read the command, with timeout (config.command_timeout)
            let mut buf = [0u8; consts::COMMAND_LENGTH];
            let command = match timeout(config.command_timeout, stream.read_exact(&mut buf)).await {
                Ok(command) => command,
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
            };

            // Check command result
            if command.is_err() {
                let is_timeout =
                    log_error(command.err(), &buf, &connection_id, &from, "COMMAND").await;
                if is_timeout {
                    stream
                        .write_all(types::Response::TimeoutError.to_bytes())
                        .await
                        .unwrap();
                } else {
                    stream
                        .write_all(types::Response::CommandError.to_bytes())
                        .await
                        .unwrap();
                }
                stream.shutdown().await.unwrap();
                return; // End the task
            }

            if let Some(command) = types::Command::from_bytes(&buf) {
                log::info!("COMMAND ({connection_id}) {command} from {from}");
                const OK_RESPONSE: types::Response = types::Response::Ok;

                match command {
                    types::Command::Open(ticket) => {
                        stream.write_all(OK_RESPONSE.to_bytes()).await.unwrap();
                        // TODO: Launch the relay
                    }
                    types::Command::Test => {
                        log::info!("TEST ({connection_id}) from {from}");
                        stream.write_all(OK_RESPONSE.to_bytes()).await.unwrap();
                        // Returns and closes the connection
                    }
                    // Stat and info are only allowed from config.allow sources (list of ips, no networks)
                    types::Command::Stat => {
                        log::info!("STAT ({connection_id}) from {from}");
                        stream.write_all(OK_RESPONSE.to_bytes()).await.unwrap();
                        // TODO: Return stats
                    }
                    types::Command::Info => {
                        log::info!("INFO ({connection_id}) from {from}");
                        stream.write_all(OK_RESPONSE.to_bytes()).await.unwrap();
                        // TODO: Return info
                    }
                    types::Command::Unknown => {
                        log_error(None, &buf, &connection_id, &from, "COMMAND").await;
                        stream
                            .write_all(types::Response::CommandError.to_bytes())
                            .await
                            .unwrap();
                    }
                }
            } else {
                let hex = to_hex(&buf);
                log_error(command.err(), &buf, &connection_id, &from, "COMMAND").await;
                log::error!("COMMAND ({connection_id}) invalid from {from}: {hex}");
                let response = types::Response::CommandError;
                stream.write_all(response.to_bytes()).await.unwrap();
            }

            stream.shutdown().await.unwrap();
        });

        task.await?;
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

async fn log_error(
    result: Option<std::io::Error>,
    buf: &[u8],
    connection_id: &str,
    from: &str,
    head: &str,
) -> bool {
    // Returns true if it was a timeout
    if let Some(e) = result {
        if e.kind() == std::io::ErrorKind::TimedOut {
            log::error!("{head} ({connection_id}) error from {from}: timed out");
            return true;
        } else {
            log::error!("{head} ({connection_id}) error from {from}: {e}");
        }
    } else {
        let hex = to_hex(&buf);
        log::error!("{head} ({connection_id}) invalid from {from}: {hex}");
    }
    false
}
