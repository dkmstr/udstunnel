use log;
use std::sync::Arc;
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
    server::TlsStream,
    TlsAcceptor,
};
use uuid;

use crate::tunnel::{relay, types};

use super::{config, consts, udsapi};
use crate::tls;

pub struct TunnelServer {
    pub udsapi: Arc<dyn udsapi::UDSApiProvider>,
    pub config: config::Config,
}

impl TunnelServer {
    pub fn new(config: &config::Config) -> Self {
        let config = config.clone();
        TunnelServer {
            udsapi: Arc::new(udsapi::HttpUDSApiProvider::new(&config)),
            config,
        }
    }

    pub fn with_provider(self, provider: Arc<dyn udsapi::UDSApiProvider>) -> Self {
        TunnelServer {
            udsapi: provider,
            config: self.config,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let certs = CertificateDer::from_pem_file(self.config.ssl_certificate.clone()).unwrap();
        let private_key: PrivateKeyDer<'_> =
            PrivateKeyDer::from_pem_file(self.config.ssl_certificate_key.clone()).unwrap();

        let protocol_versions: Vec<&rustls::SupportedProtocolVersion> =
            match self.config.ssl_min_tls_version.as_str() {
                "1.2" => vec![&TLS12],
                "1.3" => vec![&TLS13],
                _ => vec![&TLS12, &TLS13],
            };

        let server_tls_config = ServerConfig::builder_with_provider(Arc::new(
            tls::crypto_provider::provider(&self.config.ssl_ciphers),
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

        let address = if self.config.ipv6 {
            // If listen address already has brackets, don't add them
            let listen_address = if self.config.listen_address.starts_with('[') {
                self.config.listen_address.clone()
            } else {
                format!("[{}]", self.config.listen_address)
            };
            format!("{}:{}", listen_address, self.config.listen_port)
        } else {
            format!("{}:{}", self.config.listen_address, self.config.listen_port)
        };

        log::info!("Tunnel server running on {}", address);

        let listener = TcpListener::bind(address).await?;
        loop {
            let (mut stream, _) = listener.accept().await?;
            let acceptor = tls_acceptor.clone();
            let tunnel_id = uuid::Uuid::new_v4().to_string()[..13].to_string();
            let src: String = stream.peer_addr().unwrap().to_string();

            let config = self.config.clone(); // Clone the config to move it to the task
            let udsapi = self.udsapi.clone();

            let task = tokio::spawn(async move {
                log::info!("CONNECTION ({tunnel_id}) from {src}");

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
                    log_error(handshake.err(), &buf, &tunnel_id, &src, "HANDSHAKE").await;
                    stream.shutdown().await.unwrap_or_else(|e| {
                        log::warn!("Could not shutdown stream: {:?}", e);
                    });
                    return;
                }

                log::debug!("HANDSHAKE ({tunnel_id}) from {src}");

                // 2.- Upgrade the connection to TLS
                let mut stream = acceptor.accept(stream).await.unwrap();

                let command = match TunnelServer::get_command(
                    &mut stream,
                    &src,
                    config.command_timeout,
                    &tunnel_id,
                )
                .await
                {
                    None => return,
                    Some(command) => command,
                };

                let ok_response = types::Response::Ok.to_bytes();

                match command {
                    types::Command::Open(ticket) => {
                        stream.write_all(ok_response).await.unwrap();
                        relay::RelayConnection::new(tunnel_id, ticket, udsapi.clone())
                            .run(stream)
                            .await;
                    }
                    types::Command::Test => {
                        log::info!("TEST ({tunnel_id}) from {src}");
                        stream.write_all(ok_response).await.unwrap();
                        // Returns and closes the connection
                        stream.shutdown().await.unwrap();
                    }
                    // Stat and info are only allowed from config.allow sources (list of ips, no networks)
                    types::Command::Stat => {
                        log::info!("STAT ({tunnel_id}) from {src}");
                        stream.write_all(ok_response).await.unwrap();
                        // TODO: Return stats
                        stream.shutdown().await.unwrap();
                    }
                    types::Command::Info => {
                        log::info!("INFO ({tunnel_id}) from {src}");
                        stream.write_all(ok_response).await.unwrap();
                        // TODO: Return info
                        stream.shutdown().await.unwrap();
                    }
                    types::Command::Unknown => {
                        log_error(None, &buf, &tunnel_id, &src, "COMMAND").await;
                        stream
                            .write_all(types::Response::CommandError.to_bytes())
                            .await
                            .unwrap();
                        stream.shutdown().await.unwrap();
                    }
                }
            });

            task.await?;
        }
    }

    async fn get_command(
        stream: &mut TlsStream<TcpStream>,
        src: &str,
        command_timeout: std::time::Duration,
        tunnel_id: &str,
    ) -> Option<types::Command> {
        // Read the command, with timeout (config.command_timeout)
        let mut buf = [0u8; consts::COMMAND_LENGTH + consts::TICKET_LENGTH];
        let command = match timeout(command_timeout, stream.read(&mut buf)).await {
            Ok(command) => command,
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
        };

        // Check command result
        if command.is_err() {
            let is_timeout = log_error(command.err(), &buf, &tunnel_id, &src, "COMMAND").await;
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
            return None;
        }

        if let Some(command) = types::Command::from_bytes(&buf) {
            log::info!("COMMAND ({tunnel_id}) {command} from {src}");
            Some(command)
        } else {
            let hex = to_hex(&buf);
            log_error(command.err(), &buf, &tunnel_id, &src, "COMMAND").await;
            log::error!("COMMAND ({tunnel_id}) invalid from {src}: {hex}");
            let response = types::Response::CommandError;
            stream.write_all(response.to_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
            None
        }
    }
}

// Some helper functions
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
