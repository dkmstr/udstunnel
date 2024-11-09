use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::server::TlsStream;

use crate::config;

use super::udsapi;

pub(crate) async fn run(
    client_stream: TlsStream<TcpStream>,
    ticket: String,
    config: config::Config,
) -> () {
    // 1.- Try to get the ticket from UDS Server
    // 2.- If ticket is not found, log the error and return (caller will close the connection)
    // 3.- If ticket is found, we will receive (json):
    // { 'host': '....', 'port': '....', 'notify': '....' }
    // Where host it te host to connect, port is the port to connect and notify is the UDS ticket used to notification
    let src = client_stream
        .get_ref()
        .0
        .peer_addr()
        .unwrap()
        .ip()
        .to_string();

    let uds_response;
    if let Ok(response) = udsapi::request_from_uds(&config, &ticket, &src).await {
        uds_response = response;
        log::debug!("UDS Response: {:?}", uds_response);
    } else {
        log::error!("Error requesting UDS");
        return;
    }

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
}
