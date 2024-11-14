#[cfg(test)]
extern crate udstunnel;

mod fake;

use std::time::Duration;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt}, time::timeout,
};

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::tunnel::{client::connect, consts};

#[tokio::test]
async fn test_server_open_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    let reqs = server.requests.clone().unwrap();

    // Let try to connect to the server
    let client = connect("localhost", config.listen_port, false).await;

    // Note that connect already sends a handshake
    assert!(client.is_ok());

    // Send OPEN with ticket
    let ticket = [b'x'; consts::TICKET_LENGTH];
    let command = format!(
        "{}{}",
        consts::COMMAND_OPEN,
        std::str::from_utf8(&ticket).unwrap()
    );

    let mut client = client.unwrap();
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
        assert_eq!("[::1]:".to_string(), reqs[0].message[..6].to_string());
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

    server.stats.add_recv_bytes(12*4*2009);
    server.stats.add_send_bytes(1*7*1972);
    for _i in 0..947 {
        server.stats.add_global_connection();
    }
    for _i in 0..749 {
        server.stats.add_concurrent_connection();
    }

    let mut client = connect("localhost", config.listen_port, false).await.unwrap();
    let command = format!("{}{}", consts::COMMAND_STATS, config.secret);
    client.write_all(command.as_bytes()).await.unwrap();
    let mut buffer = [0; 8192];
    let n = client.read(&mut buffer).await.unwrap();
    let response = std::str::from_utf8(&buffer[..n]).unwrap();

    let stats = response.split(';').collect::<Vec<&str>>();
    assert_eq!(stats.len(), 4);
    assert_eq!(stats[0], server.stats.get_concurrent_connections().to_string());
    assert_eq!(stats[1], server.stats.get_globals_connections().to_string());
    assert_eq!(stats[2], server.stats.get_sent_bytes().to_string());
    assert_eq!(stats[3], server.stats.get_recv_bytes().to_string());

}


#[tokio::test]
async fn test_invalid_command() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

    // Let try to connect to the server
    let client = connect("localhost", config.listen_port, false).await;

    // Note that connect already sends a handshake
    assert!(client.is_ok());

    // Send an invalid command
    let command = "INVALID_COMMAND";
    let mut client = client.unwrap();
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
