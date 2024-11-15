#[cfg(test)]
extern crate udstunnel;

mod fake;

use std::time::Duration;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use udstunnel::tunnel::consts;

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

#[tokio::test]
async fn test_server_listens() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let _client = fake::client::open_client_no_handshake(config.listen_port).await;

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            panic!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_server_handshake() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    // Note that connect already sends a handshake message
    let mut client = fake::client::open_client_with_handshake(config.listen_port).await;

    // Server expects a command, but no command will be isued.
    // Should work anyway
    client.shutdown().await.unwrap();

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            panic!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_server_handshake_timeout() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let mut client = fake::client::open_client_no_handshake(config.listen_port).await;

    // Let's wait a bit to allow the server to process the handshake
    tokio::time::sleep(config.handshake_timeout + tokio::time::Duration::from_millis(200)).await;

    // If timeout, the socket will be closed, could not read from it
    let mut buf = vec![0u8; 1];
    let response_size = timeout(Duration::from_secs(2), client.read_buf(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(response_size == consts::RESPONSE_ERROR_TIMEOUT.len());

    // assert!(client.is_err());

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            panic!("Error: {:?}", e);
        }
    }
}
