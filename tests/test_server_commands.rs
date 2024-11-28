#[cfg(test)]
extern crate udstunnel;

mod fake;

use std::time::Duration;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::tunnel::consts;

#[tokio::test]
async fn test_server_test_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let mut client = fake::client::open_client_with_handshake(config.listen_port).await;

    // Send TEST
    let command = consts::COMMAND_TEST;
    client.write_all(command.as_bytes()).await.unwrap();
    // Should receive a RESPONSE_OK
    let mut buffer = [0; 8192];
    let n = client.read(&mut buffer).await.unwrap();
    assert!(n == consts::RESPONSE_OK.len());
    assert_eq!(
        std::str::from_utf8(&buffer[..n]).unwrap(),
        consts::RESPONSE_OK
    );

    client.shutdown().await.unwrap();

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_server_open_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    let reqs = server.requests.clone().unwrap();

    // Let try to connect to the server
    let mut client = fake::client::open_client_with_handshake(config.listen_port).await;

    // Send OPEN with ticket
    let ticket = [b'x'; consts::TICKET_LENGTH];
    let command = format!(
        "{}{}",
        consts::COMMAND_OPEN,
        std::str::from_utf8(&ticket).unwrap()
    );

    client.write_all(command.as_bytes()).await.unwrap();
    // Should receive a RESPONSE_OK
    let mut buffer = [0; 8192];
    let n = client.read(&mut buffer).await.unwrap();
    assert!(n == consts::RESPONSE_OK.len());
    assert_eq!(
        std::str::from_utf8(&buffer[..n]).unwrap(),
        consts::RESPONSE_OK
    );

    // Check the request
    // Note: Reqs has a lock that will be hold on shutdown
    // so we need to ensure lock is released before calling shutdonw
    {
        let reqs = reqs.lock().unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].ticket, std::str::from_utf8(&ticket).unwrap());
        assert_eq!("::1".to_string(), reqs[0].message[..3].to_string());
        assert_eq!(reqs[0].query_params, None);
    }

    client.shutdown().await.unwrap();

    server.abort();

    match timeout(Duration::from_secs(4), server.server_handle).await {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_server_stats() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    server.stats.add_recv_bytes(12 * 4 * 2009);
    server.stats.add_send_bytes(1 * 7 * 1972);
    for _i in 0..947 {
        server.stats.add_global_connection();
    }
    for _i in 0..749 {
        server.stats.add_concurrent_connection();
    }

    let mut client = fake::client::open_client_with_handshake(config.listen_port).await;

    let command = format!("{}{}", consts::COMMAND_STATS, config.secret);
    client.write_all(command.as_bytes()).await.unwrap();
    let mut buffer = [0; 8192];
    let n = client.read(&mut buffer).await.unwrap();
    let response = std::str::from_utf8(&buffer[..n]).unwrap();

    let stats = response.split(';').collect::<Vec<&str>>();
    assert_eq!(stats.len(), 4);
    assert_eq!(
        stats[0],
        server.stats.get_concurrent_connections().to_string()
    );
    assert_eq!(stats[1], server.stats.get_globals_connections().to_string());
    assert_eq!(stats[2], server.stats.get_sent_bytes().to_string());
    assert_eq!(stats[3], server.stats.get_recv_bytes().to_string());
}

#[tokio::test]
async fn test_invalid_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let mut client = fake::client::open_client_with_handshake(config.listen_port).await;

    // Send an invalid command
    let command = "INVALID_COMMAND";
    client.write_all(command.as_bytes()).await.unwrap();
    // Should receive a RESPONSE_ERROR
    let mut buffer = [0; 8192];
    let n = client.read(&mut buffer).await.unwrap();
    assert!(n == consts::RESPONSE_ERROR_COMMAND.len());
    assert_eq!(
        std::str::from_utf8(&buffer[..n]).unwrap(),
        consts::RESPONSE_ERROR_COMMAND
    );

    client.shutdown().await.unwrap();

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_timedout_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let mut client = fake::client::open_client_no_handshake(config.listen_port).await;

    // No command will be issued

    // Should receive a TIMEOUT
    let mut buffer = [0; 8192];
    let n = timeout(Duration::from_secs(4), client.read(&mut buffer))
        .await
        .unwrap() // Timeout
        .unwrap(); // Result
    assert!(n == consts::RESPONSE_ERROR_TIMEOUT.len());
    assert_eq!(
        std::str::from_utf8(&buffer[..n]).unwrap(),
        consts::RESPONSE_ERROR_TIMEOUT
    );

    // Socket should be closed (correctly, so no error)
    assert!(
        timeout(Duration::from_secs(4), client.read(&mut buffer))
            .await
            .unwrap()
            .unwrap()
            == 0
    );

    client.shutdown().await.unwrap();

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}
