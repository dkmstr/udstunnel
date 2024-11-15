#[cfg(test)]
extern crate udstunnel;

mod fake;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
};

use udstunnel::tunnel::consts;

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

#[tokio::test]
async fn test_server_to_remote() {
    let config = fake::config::read().await;
    let server = fake::tunnel_server::TunnelServer::create(&config, true).await;

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
    let mut buffer = [0; 128];
    let n = client.read(&mut buffer).await.unwrap();
    assert!(n == consts::RESPONSE_OK.len());
    assert_eq!(
        std::str::from_utf8(&buffer[..n]).unwrap(),
        consts::RESPONSE_OK
    );

    // Now write some data, will return same data
    let data = [b'x'; 128];
    client.write_all(&data).await.unwrap();
    let mut buffer = [0; 1024];
    let n = client.read(&mut buffer).await.unwrap();
    assert_eq!(n, data.len());
    assert_eq!(&buffer[..n], &data);

    server.abort();

    match server.server_handle.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            panic!("Error: {:?}", e);
        }
    }
}
