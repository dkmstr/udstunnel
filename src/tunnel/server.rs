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

use super::{config, consts, event, stats, udsapi};
use crate::tls;

pub struct TunnelServer {
    pub udsapi: Arc<dyn udsapi::UDSApiProvider>,
    pub config: config::Config,
    pub stats: Arc<stats::Stats>,
}

impl TunnelServer {
    pub fn new(config: &config::Config, stats: Arc<stats::Stats>) -> Self {
        let config = config.clone();
        TunnelServer {
            udsapi: Arc::new(udsapi::HttpUDSApiProvider::new(&config)),
            config,
            stats,
        }
    }

    pub fn with_provider(self, provider: Arc<dyn udsapi::UDSApiProvider>) -> Self {
        TunnelServer {
            udsapi: provider,
            config: self.config,
            stats: self.stats,
        }
    }

    pub async fn run(self, stop_event: event::Event) -> Result<(), Box<dyn std::error::Error>> {
        let certs = CertificateDer::from_pem_file(self.config.ssl_certificate.clone()).unwrap();
        let private_key: PrivateKeyDer<'_> =
            PrivateKeyDer::from_pem_file(self.config.ssl_certificate_key.clone()).unwrap();

        let protocol_versions: Vec<&rustls::SupportedProtocolVersion> =
            match self.config.ssl_min_tls_version.as_str() {
                "1.2" => vec![&TLS12, &TLS13],
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
            let mut stream;
            let check_stop_event = stop_event.clone();
            tokio::select! {
                _ = check_stop_event => {
                    break;
                }
                listener = listener.accept() => {
                    stream = listener?.0;
                }
            };

            let acceptor = tls_acceptor.clone();
            let tunnel_id = uuid::Uuid::new_v4().to_string()[..13].to_string();
            let src: String = stream.peer_addr().unwrap().to_string();

            let config = self.config.clone(); // Clone the config to move it to the task
            let udsapi = self.udsapi.clone();
            let stats = self.stats.clone();

            // A new connection, increment the counter
            stats.add_global_connection();

            let relay_stop_event = stop_event.clone();
            tokio::spawn(async move {
                log::info!("CONNECTION ({tunnel_id}) from {src}");

                let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

                // 1.- Read the handshake (with timeout)
                let handshake =
                    match timeout(config.handshake_timeout, stream.read_exact(&mut buf)).await {
                        Ok(handshake) => handshake,
                        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                    };

                // If no valid, even if timeout, close the connection and log the error
                if handshake.is_err() || buf != consts::HANDSHAKE_V1 {
                    if handshake.is_err() {
                        stream
                            .write_all(types::Response::TimeoutError.to_bytes())
                            .await
                            .unwrap_or_default();
                    } else {
                        stream
                            .write_all(types::Response::HandshakeError.to_bytes())
                            .await
                            .unwrap_or_default();
                    }
                    log_error(handshake.err(), &buf, &tunnel_id, &src, "HANDSHAKE").await;
                    // If timeout, send a timeout response
                    stream.shutdown().await.unwrap_or_default(); // Ignore error
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

                match command {
                    types::Command::Open(ticket) => {
                        relay::RelayConnection::new(
                            tunnel_id,
                            ticket,
                            udsapi.clone(),
                            stats.clone(),
                        )
                        .run(stream, relay_stop_event.clone())
                        .await;
                    }
                    types::Command::Test => {
                        log::info!("TEST ({tunnel_id}) from {src}");
                        stream
                            .write_all(types::Response::Ok.to_bytes())
                            .await
                            .unwrap();
                        // Returns and closes the connection
                        stream.shutdown().await.unwrap();
                    }
                    // Stat and info are only allowed from config.allow sources (list of ips, no networks)
                    types::Command::Stats(secret) => {
                        log::info!("STATS ({tunnel_id}) from {src}");
                        // Should be of a valid source and secret
                        let ip = stream.get_ref().0.peer_addr().unwrap().ip().to_string();
                        // Ip does not have brackets, if it's an IPv6
                        if !config.allow.is_empty()
                            && (!config.allow.contains(&ip) || secret != config.secret)
                        {
                            stream
                                .write_all(types::Response::ForbiddenError.to_bytes())
                                .await
                                .unwrap();
                            return;
                        }

                        let stats = format!(
                            "{};{};{};{}",
                            stats.get_concurrent_connections(),
                            stats.get_globals_connections(),
                            stats.get_sent_bytes(),
                            stats.get_recv_bytes()
                        );
                        stream.write_all(stats.as_bytes()).await.unwrap();

                        stream.shutdown().await.unwrap();
                    }
                    types::Command::Unknown => {
                        log_error(None, &buf, &tunnel_id, &src, "COMMAND").await;
                        stream
                            .write_all(types::Response::CommandError.to_bytes())
                            .await
                            .unwrap_or_default();

                        stream.shutdown().await.unwrap_or_default(); // Ignore error
                    }
                }
            });

            // The task will run in the background, we don't need to await it
        }
        Ok(())
    }

    async fn get_command(
        stream: &mut TlsStream<TcpStream>,
        src: &str,
        command_timeout: std::time::Duration,
        tunnel_id: &str,
    ) -> Option<types::Command> {
        // Read the command, with timeout (config.command_timeout)
        let mut buf = [0u8; 128]; // 128 bytes should be enough for a command and a ticket/secret
        let cmd_read_result = match timeout(command_timeout, stream.read(&mut buf)).await {
            Ok(read_result) => read_result,
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
        };

        // Check command result
        if cmd_read_result.is_err() {
            let is_timeout =
                cmd_read_result.as_ref().unwrap_err().kind() == std::io::ErrorKind::TimedOut;
            log_error(cmd_read_result.err(), &buf, &tunnel_id, &src, "COMMAND").await;
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

        let size = cmd_read_result.as_ref().unwrap();
        let buf = &buf[..*size];

        if let Some(command) = types::Command::from_bytes(&buf) {
            log::info!("COMMAND ({tunnel_id}) {command} from {src}");
            Some(command)
        } else {
            let hex = to_hex(&buf);
            log_error(cmd_read_result.err(), &buf, &tunnel_id, &src, "COMMAND").await;
            log::error!("COMMAND ({tunnel_id}) invalid from {src}: {hex}");
            let response = types::Response::CommandError;
            stream
                .write_all(response.to_bytes())
                .await
                .unwrap_or_default(); // Ignore error, returning error
            stream.shutdown().await.unwrap_or_default(); // Ignore error,
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
) -> () {
    // Returns true if it was a timeout
    if let Some(e) = result {
        if e.kind() == std::io::ErrorKind::TimedOut {
            log::error!("{head} ({connection_id}) error from {from}: timed out");
        } else {
            log::error!("{head} ({connection_id}) error from {from}: {e}");
        }
    } else {
        let hex = to_hex(&buf);
        log::error!("{head} ({connection_id}) invalid from {from}: {hex}");
    }
}
