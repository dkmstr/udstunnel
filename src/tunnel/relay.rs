use std::{io, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::server::TlsStream;

use super::udsapi;

pub struct RelayConnection {
    pub tunnel_id: String,
    pub ticket: String,
    pub udsapi: Arc<dyn udsapi::UDSApiProvider>,

    pub src: String, // Source IP/Port
    pub dst: String, // Destination IP/Port
    pub notify_ticket: String,
}

impl RelayConnection {
    pub fn new(tunnel_id: String, ticket: String, udsapi: Arc<dyn udsapi::UDSApiProvider>) -> Self {
        Self {
            tunnel_id,
            ticket,
            udsapi,
            src: String::new(),
            dst: String::new(),
            notify_ticket: String::new(),
        }
    }

    pub(crate) async fn run(
        &mut self,
        mut client_stream: TlsStream<TcpStream>, // move value
    ) -> () {
        // 1.- Try to get the ticket from UDS Server
        // 2.- If ticket is not found, log the error and return (caller will close the connection)
        // 3.- If ticket is found, we will receive (json):
        // { 'host': '....', 'port': '....', 'notify': '....' }
        // Where host it te host to connect, port is the port to connect and notify is the UDS ticket used to notification
        let src_peer_addr = client_stream.get_ref().0.peer_addr().unwrap();

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
                client_stream.write_all(response.as_bytes()).await.unwrap();
                client_stream.shutdown().await.unwrap();
                return;
            }
        }

        self.dst = format!("{}:{}", uds_response.host, uds_response.port);
        self.notify_ticket = uds_response.notify;

        // Open the connection to the destination server (server stream)
        let server = format!("{}:{}", uds_response.host, uds_response.port);
        let server_stream = TcpStream::connect(server).await.unwrap();
        let (mut server_reader, mut server_writer) = server_stream.into_split();

        // Split the client stream into reader and writer
        let (mut client_reader, mut client_writer) = tokio::io::split(client_stream);

        let server_to_client = tokio::task::spawn(async move {
            // Using a buf on heap so transfer between tasks is faster
            // Only need to transfer the pointer, not the data as in the case of [u8; 1024]
            let mut buf = vec![0; 1024];
            loop {
                //let _ = reader.readable().await.unwrap();
                // match reader.try_read(&mut buf) {
                match server_reader.read_buf(&mut buf).await {
                    Ok(0) => {
                        break;
                    }
                    Ok(n) => {
                        client_writer.write_all(&buf[..n]).await.unwrap();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        // Last one, move value
                        println!("Error reading from relay: {:?}", e);
                        break;
                    }
                }
            }
        });

        let client_to_server = tokio::task::spawn(async move {
            // Using a buf on heap so transfer between tasks is faster
            // Only need to transfer the pointer, not the data as in the case of [u8; 1024]
            let mut buf = vec![0; 1024];
            loop {
                match client_reader.read_buf(&mut buf).await {
                    Ok(0) => {
                        break;
                    }
                    Ok(n) => {
                        server_writer.write_all(&buf[..n]).await.unwrap();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        // Last one, move value
                        println!("Error reading from relay: {:?}", e);
                        break;
                    }
                }
            }
        });
        // As soon as one of the tasks completes, the other task will be cancelled
        tokio::select! {
            res = client_to_server => {
                log::debug!("client_to_server task completed: {:?}", res);
            }
            res = server_to_client => {
                log::debug!("Write task completed: {:?}", res);
            }
        }
        // As soon as the tasks are completed, the connection will be closed
        // because the streams halves will get out of scope, so they will be dropped

        // Notify the end to UDS and log it
        self.notify_end().await;
    }

    async fn notify_end(&mut self) -> () {
        if !self.notify_ticket.is_empty() {
            log::info!(
                "TERMINATED ({}) {} to {}, s:{}, r:{}, t:{}",
                self.tunnel_id,
                self.src,
                self.dst,
                0u64,
                0u64,
                0u64,
                //self.stats_manager.local.sent,
                //self.stats_manager.local.recv,
                //self.stats_manager.elapsed_time,
            );
            // Send the notification to UDS
            let _ = self.udsapi.notify_end(&self.ticket, 0, 0).await;
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
