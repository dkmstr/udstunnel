#[cfg(test)]
extern crate udstunnel;

mod fake;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::tunnel::{client::connect, consts};

#[tokio::test]
async fn test_server_listens() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Check it's listening...
    match timeout(
        Duration::from_millis(200),
        TcpStream::connect(format!("{}:{}", "localhost", config.listen_port)),
    )
    .await
    {
        Ok(conn) => {
            conn.unwrap().shutdown().await.unwrap();
        }
        Err(e) => {
            panic!("Error connecting to server: {:?}", e);
        }
    }

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
    let client = connect("localhost", config.listen_port, false).await;
    // Note that connect already sends a handshake message
    assert!(client.is_ok());

    // Server expects a command, but no command will be isued.
    // Should work anyway
    client.unwrap().shutdown().await.unwrap();

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
    // Check it's listening...
    let client = match timeout(
        Duration::from_millis(200),
        TcpStream::connect(format!("{}:{}", "localhost", config.listen_port)),
    )
    .await
    {
        Ok(conn) => conn,
        Err(e) => {
            panic!("Error connecting to server: {:?}", e);
        }
    };

    // Note that connect already sends a handshake message
    assert!(client.is_ok());

    // Let's wait a bit to allow the server to process the handshake
    tokio::time::sleep(consts::HANDSHAKE_TIMEOUT + tokio::time::Duration::from_millis(200)).await;

    // If timeout, the socket will be closed, could not read from it
    let mut buf = vec![0u8; 1];
    assert!(client.unwrap().read_buf(&mut buf).await.unwrap() == 0);

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
