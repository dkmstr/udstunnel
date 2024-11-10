#[cfg(test)]
extern crate udstunnel;

mod fake;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
};

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::tunnel::{client::connect, consts};

#[tokio::test]
async fn test_server_open_command() {
    let config = fake::config::read().await;
    let (server, reqs) = fake::tunnel_server::create(&config, true).await;

    let reqs = reqs.unwrap(); // Expects always to be Some, or fails

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
    let reqs = reqs.lock().unwrap();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].ticket, std::str::from_utf8(&ticket).unwrap());
    assert_eq!("[::1]:".to_string(), reqs[0].message[..6].to_string());
    assert_eq!(reqs[0].query_params, None);

    client.shutdown().await.unwrap();

    server.abort();

    match server.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            assert_eq!(e.is_cancelled(), true);
        }
    }
}

#[tokio::test]
async fn test_invalid_command() {
    let config = fake::config::read().await;
    let (server, _) = fake::tunnel_server::create(&config, true).await;

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

    match server.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            assert_eq!(e.is_cancelled(), true);
        }
    }
}
