use std::{io, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::server::TlsStream;

use super::{event, stats, types, udsapi};

pub struct RelayConnection {
    pub tunnel_id: String,
    pub ticket: String,
    pub udsapi: Arc<dyn udsapi::UDSApiProvider>,

    pub src: String, // Source IP/Port
    pub dst: String, // Destination IP/Port
    pub notify_ticket: String,

    pub global_stats: Arc<stats::Stats>,
    pub local_stats: Arc<stats::Stats>,
}

impl RelayConnection {
    pub fn new(
        tunnel_id: String,
        ticket: String,
        udsapi: Arc<dyn udsapi::UDSApiProvider>,
        stats: Arc<stats::Stats>,
    ) -> Self {
        Self {
            tunnel_id,
            ticket,
            udsapi,
            src: String::new(),
            dst: String::new(),
            notify_ticket: String::new(),
            global_stats: stats.clone(),
            local_stats: Arc::new(stats::Stats::new()),
        }
    }

    pub(crate) async fn run(
        &mut self,
        mut client_stream: TlsStream<TcpStream>, // move value
        stop_event: event::Event,
    ) -> () {
        // 1.- Try to get the ticket from UDS Server
        // 2.- If ticket is not found, log the error and return (caller will close the connection)
        // 3.- If ticket is found, we will receive (json):
        // { 'host': '....', 'port': '....', 'notify': '....' }
        // Where host it te host to connect, port is the port to connect and notify is the UDS ticket used to notification
        let src_peer_addr =
            client_stream
                .get_ref()
                .0
                .peer_addr()
                .unwrap_or(std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                    0,
                ));
        if src_peer_addr.ip().is_unspecified() {
            log::error!("Error getting peer address!");
            return;
        }

        if src_peer_addr.ip().to_string().contains(':') {
            self.src = format!("[{}]:{}", src_peer_addr.ip(), src_peer_addr.port());
        } else {
            self.src = format!("{}:{}", src_peer_addr.ip(), src_peer_addr.port());
        }

        let uds_response;
        if let Ok(response) = self.udsapi.get_ticket(&self.ticket, &self.src).await {
            uds_response = response;
            log::debug!("UDS Response: {:?}", uds_response);
        } else {
            log::error!("Error requesting UDS");
            return;
        }

        // If host starts with #, it's a command, process it and return
        if uds_response.host.starts_with('#') {
            log::debug!("Command received: {}", uds_response.host);
            if let Some(response) = self.execute_command(&uds_response.host).await {
                log::debug!("Command response: {}", response);
                // Ignore errors, we are closing the connection
                client_stream
                    .write_all(response.as_bytes())
                    .await
                    .unwrap_or_default();
                client_stream.shutdown().await.unwrap_or_default();
                return;
            }
        }

        self.dst = format!("{}:{}", uds_response.host, uds_response.port);
        self.notify_ticket = uds_response.notify;

        // Open the connection to the destination server (server stream)
        let server = format!("{}:{}", uds_response.host, uds_response.port);

        log::info!(
            "OPEN TUNNEL ({}) FROM {} to {}",
            self.tunnel_id,
            self.src,
            self.dst
        );

        let server_stream = match TcpStream::connect(server.clone()).await {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("CONNECTION FAILED ({}): {:?}", self.tunnel_id, e);
                client_stream
                    .write_all(types::Response::ConnectError.to_bytes())
                    .await
                    .unwrap();
                client_stream.shutdown().await.unwrap_or_default(); // Close the client connection
                return;
            }
        };
        client_stream
            .write_all(types::Response::Ok.to_bytes())
            .await
            .unwrap();

        let (mut server_reader, mut server_writer) = server_stream.into_split();

        // Split the client stream into reader and writer
        let (mut client_reader, mut client_writer) = tokio::io::split(client_stream);

        // Current connections counter
        self.global_stats.add_concurrent_connection();

        let local_tasks_stopper = event::Event::new();

        let global_stats = self.global_stats.clone();
        let local_stats = self.local_stats.clone();

        let global_stopper = stop_event.clone();
        let local_stopper = local_tasks_stopper.clone();
        let server_to_client = tokio::task::spawn(async move {
            let mut buf = vec![0; 1024];
            log::debug!("Starting server_to_client task");
            loop {
                tokio::select! {
                    _ = global_stopper.clone() => {
                        log::debug!("Stopping server_to_client task");
                        break;
                    }
                    _ = local_stopper.clone() => {
                        log::debug!("Stopping server_to_client task");
                        break;
                    }
                    read_result = server_reader.read_buf(&mut buf) => {
                        match read_result {
                            Ok(0) => {
                                break;
                            }
                            Ok(n) => {
                                // Ad to global and local stats
                                global_stats.add_send_bytes(n as u64);
                                local_stats.add_send_bytes(n as u64);

                                let mut error: Option<()> = None;
                                client_writer.write_all(&buf[..n]).await.unwrap_or_else(|_| error = Some(()));
                                if error.is_some() {
                                    log::error!("ERROR writing to client");
                                    break;
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => {
                                // Last one, move value
                                log::error!("ERROR from server: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        let global_stats = self.global_stats.clone();
        let local_stats = self.local_stats.clone();

        let global_stopper = stop_event.clone();
        let local_stopper = local_tasks_stopper.clone();

        let client_to_server = tokio::task::spawn(async move {
            let mut buf = vec![0; 1024];
            log::debug!("Starting client_to_server task");
            loop {
                tokio::select! {
                    _ = global_stopper.clone() => {
                        log::debug!("Stopping client_to_server task");
                        break;
                    }
                    _ = local_stopper.clone() => {
                        log::debug!("Stopping client_to_server task");
                        break;
                    }
                    read_result = client_reader.read_buf(&mut buf) => {
                        match read_result {
                            Ok(0) => {
                                break;
                            }
                            Ok(n) => {
                                // Ad to global and local stats
                                global_stats.add_recv_bytes(n as u64);
                                local_stats.add_recv_bytes(n as u64);
                                let mut error: Option<()> = None;
                                server_writer.write_all(&buf[..n]).await.unwrap_or_else(|_| error = Some(()));
                                if error.is_some() {
                                    log::error!("ERROR writing to server");
                                    break;
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => {
                                // Last one, move value
                                log::error!("ERROR from client: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        // As soon as one of the tasks completes, the other task will be cancelled

        log::debug!("Waiting for any to complete");
        tokio::select! {
            res = client_to_server => {
                log::debug!("client_to_server task completed: {:?}", res);
            }
            res = server_to_client => {
                log::debug!("Write task completed: {:?}", res);
            }
        }
        // Ensure the other task is also stopped
        local_tasks_stopper.set();

        // And update the concurrent connections
        self.global_stats.sub_concurrent_connection();
        
        // As soon as one or the tasks is completed, the connection will be closed
        // Ant the other task will be cancelled
        // because the streams halves will get out of scope, so they will be dropped

        // Notify the end to UDS and log it
        log::debug!("Notifying end to UDS");
        self.notify_end().await;
        log::debug!("End of tunnel relay");
    }

    async fn notify_end(&mut self) -> () {
        if !self.notify_ticket.is_empty() {
            log::info!(
                "TERMINATED ({}) {} to {}, s:{}, r:{}, t:{}",
                self.tunnel_id,
                self.src,
                self.dst,
                self.local_stats.get_sent_bytes(),
                self.local_stats.get_recv_bytes(),
                self.local_stats.get_duration().as_secs()
            );
            // Send the notification to UDS
            let _ = self
                .udsapi
                .notify_end(
                    &self.ticket,
                    self.local_stats.get_sent_bytes(),
                    self.local_stats.get_recv_bytes(),
                    self.local_stats.get_duration(),
                )
                .await;
            self.notify_ticket.clear(); // Clean up so no more notifications
        } else {
            log::info!("TERMINATED ({}) {}", self.tunnel_id, self.src);
        }

        // self.stats_manager.close();
        // self.owner.finished.set();
    }

    async fn execute_command(&self, command: &str) -> Option<String> {
        let command = command.trim_start_matches('#');
        match command {
            "close" => None,
            _ => {
                log::info!("Command received: {}", command);
                None
            }
        }
        // Execute the command
        // let output = Command::new("sh")
        //     .arg("-c")
        //     .arg(command)
        //     .output()
        //     .expect("failed to execute process");
        // log::info!("Command output: {}", String::from_utf8_lossy(&output.stdout));
    }
}
