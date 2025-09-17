use anyhow::{Context, Result};
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

// let acceptor = tls_acceptor.clone();
// let tunnel_id = uuid::Uuid::new_v4().to_string()[..13].to_string();
// let src: String = stream.peer_addr().unwrap().to_string();

// let config = self.config.clone(); // Clone the config to move it to the task
// let udsapi = self.udsapi.clone();
// let stats = self.stats.clone();

struct Connection {
    acceptor: TlsAcceptor,
    stream: Option<TcpStream>,
    tunnel_id: String,
    config: config::Config,
    udsapi: Arc<dyn udsapi::UDSApiProvider>,
    stats: Arc<stats::Stats>,
    stop_event: event::Event,
}

impl Connection {
    pub fn new(
        acceptor: TlsAcceptor,
        stream: TcpStream,
        tunnel_id: String,
        config: config::Config,
        udsapi: Arc<dyn udsapi::UDSApiProvider>,
        stats: Arc<stats::Stats>,
        stop_event: event::Event,
    ) -> Self {
        Connection {
            acceptor,
            stream: Some(stream),
            tunnel_id,
            config,
            udsapi,
            stats,
            stop_event,
        }
    }

    async fn process(&mut self) -> Result<()> {
        // A new connection, increment the counter
        self.stats.add_global_connection();

        let mut stream = self.stream.take().context("Stream already taken")?;

        let src_ip = stream.peer_addr().unwrap().ip().to_string();

        log::info!("CONNECTION ({}) from {}", self.tunnel_id, src_ip);

        let mut buf = vec![0u8; consts::HANDSHAKE_V1.len()];

        // 1.- Read the handshake (with timeout)
        let handshake = match timeout(
            self.config.handshake_timeout,
            stream.read_exact(&mut buf),
        )
        .await
        {
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
            log_error(
                handshake.err(),
                &buf,
                &self.tunnel_id,
                &src_ip,
                "HANDSHAKE",
            )
            .await;
            // If timeout, send a timeout response
            stream.shutdown().await.unwrap_or_default(); // Ignore error
            return Ok(());
        }

        log::debug!("HANDSHAKE ({}) from {}", self.tunnel_id, src_ip);

        // 2.- Upgrade the connection to TLS
        let mut stream = self.acceptor.accept(stream).await.unwrap();

        let command = match TunnelServer::get_command(
            &mut stream,
            &src_ip,
            self.config.command_timeout,
            &self.tunnel_id,
        )
        .await
        {
            Ok(command) => command,
            Err(err) => {
                log::debug!(
                    "COMMAND ({}) read error from {}: {:?}",
                    self.tunnel_id,
                    src_ip,
                    err
                );
                return Ok(());
            }
        };

        match command {
            types::Command::Open(ticket) => {
                let mut relay = relay::RelayConnection::new(
                    self.tunnel_id.clone(),
                    ticket,
                    self.udsapi.clone(),
                    self.stats.clone(),
                );
                let relay_stop_event = self.stop_event.clone();
                if let Err(e) = relay.run(stream, relay_stop_event.clone()).await {
                    log::error!(
                        "RELAY ({}) error from {}: {:?}",
                        self.tunnel_id,
                        src_ip,
                        e
                    );
                    Err(anyhow::anyhow!(e))
                } else {
                    Ok(())
                }
            }
            types::Command::Test => {
                log::info!("TEST ({}) from {}", self.tunnel_id, src_ip);
                stream
                    .write_all(types::Response::Ok.to_bytes())
                    .await
                    .unwrap();
                // Returns and closes the connection
                stream
                    .shutdown()
                    .await
                    .context("Error shutting down stream")?;
                Ok(())
            }
            // Stat and info are only allowed from config.allow sources (list of ips, no networks)
            types::Command::Stats(secret) => {
                log::info!("STATS ({}) from {}", self.tunnel_id, src_ip);
                // Should be of a valid source and secret
                let ip = stream
                    .get_ref()
                    .0
                    .peer_addr()
                    .unwrap()
                    .ip()
                    .to_string();
                // Ip does not have brackets, if it's an IPv6
                if !self.config.allow.is_empty()
                    && (!self.config.allow.contains(&ip) || secret != self.config.secret)
                {
                    stream
                        .write_all(types::Response::ForbiddenError.to_bytes())
                        .await
                        .context("Error writing forbidden response")?;
                }

                let stats = format!(
                    "{};{};{};{}",
                    self.stats.get_concurrent_connections(),
                    self.stats.get_globals_connections(),
                    self.stats.get_sent_bytes(),
                    self.stats.get_recv_bytes()
                );
                stream
                    .write_all(stats.as_bytes())
                    .await
                    .context("Error writing stats")?;

                stream
                    .shutdown()
                    .await
                    .context("Error shutting down stream")?;
                Ok(())
            }
            types::Command::Unknown => {
                log_error(None, &buf, &self.tunnel_id, &src_ip, "COMMAND").await;
                stream
                    .write_all(types::Response::CommandError.to_bytes())
                    .await
                    .unwrap_or_default();

                stream.shutdown().await.context("Error shutting down stream")?;
                Err(anyhow::anyhow!("Invalid command"))
            }
        }
    }
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

    pub async fn run(self, stop_event: event::Event) -> Result<()> {
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
            let stream;
            let check_stop_event = stop_event.clone();
            tokio::select! {
                _ = check_stop_event => {
                    break;
                }
                listener = listener.accept() => {
                    stream = listener?.0;
                }
            };


            let mut connection = Connection::new(
                tls_acceptor.clone(),
                stream,
                uuid::Uuid::new_v4().to_string()[..13].to_string(),
                self.config.clone(),
                self.udsapi.clone(),
                self.stats.clone(),
                stop_event.clone(),
            );

            // Process the connection in a new task
            tokio::spawn(async move {
                if let Err(e) = connection.process().await {
                    log::error!("Connection error: {:?}", e);
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
    ) -> Result<types::Command, std::io::Error> {
        // Read the command, with timeout (config.command_timeout)
        let mut buf = [0u8; 128]; // 128 bytes should be enough for a command and a ticket/secret
        let cmd_read_result = match timeout(command_timeout, stream.read(&mut buf)).await {
            Ok(read_result) => read_result,
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
        };

        // Check command result
        if let Err(err) = &cmd_read_result {
            log::debug!("COMMAND ({tunnel_id}) read error from {src}: {:?}", err);
            if err.kind() == std::io::ErrorKind::TimedOut {
                log::debug!("COMMAND ({tunnel_id}) read timeout from {src}");
                stream
                    .write_all(types::Response::TimeoutError.to_bytes())
                    .await?;
            } else {
                stream
                    .write_all(types::Response::CommandError.to_bytes())
                    .await?;
            }
            stream.shutdown().await?;
            return Err(std::io::Error::new(err.kind(), err.to_string()));
        }

        let size = cmd_read_result.as_ref().unwrap();
        let buf = &buf[..*size];

        if let Ok(command) = types::Command::from_bytes(buf) {
            log::info!("COMMAND ({tunnel_id}) {command} from {src}");
            Ok(command)
        } else {
            let hex = to_hex(buf);
            log_error(cmd_read_result.err(), buf, tunnel_id, src, "COMMAND").await;
            log::error!("COMMAND ({tunnel_id}) invalid from {src}: {hex}");
            let response = types::Response::CommandError;
            stream
                .write_all(response.to_bytes())
                .await
                .unwrap_or_default(); // Ignore error, returning error
            stream.shutdown().await.unwrap_or_default(); // Ignore error,
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid command",
            ))
        }
    }
}

// Some helper functions
fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(16)
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
        let hex = to_hex(buf);
        log::error!("{head} ({connection_id}) invalid from {from}: {hex}");
    }
}
