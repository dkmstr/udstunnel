#[cfg(test)]
extern crate udstunnel;

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, TcpListener},
    task::JoinHandle,
    time::{timeout, Duration},
};

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::{
    config,
    tunnel::{self, client::connect, consts, server::launch},
};

async fn get_config() -> config::Config {
    let mut config = config::ConfigLoader::new()
        .with_filename("tests/udstunnel.conf")
        .load()
        .unwrap();

    // Get a free por for the configuration, so we can run multiple tests
    match TcpListener::bind(format!("{}:0", config.listen_address)).await {
        Ok(listener) => {
            let addr = listener.local_addr().unwrap();
            config.listen_port = addr.port();
        }
        Err(e) => {
            panic!("Error binding listener: {:?}", e);
        }
    }


    tunnel::log::setup(&None, &config.loglevel);
    config
}

async fn create_server() -> (JoinHandle<()>, config::Config) {
    let config = get_config().await;

    let launch_config = config.clone();
    let server = tokio::spawn(async move {
        let result = launch(launch_config).await;
        assert!(result.is_ok());
    });

    // Should be listening on configure port, let's wait a bit to
    // allow tokio to start the server
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    (server, config)
}

#[tokio::test]
async fn test_launch_listens() {
    let (server, config) = create_server().await;

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

    // // Let try to connect to the server
    // let client = connect("localhost", 4443, false).await;
    // assert!(client.is_ok());
    // client.unwrap().shutdown().await.unwrap();

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
async fn test_launch_handshake() {
    let (server, config) = create_server().await;

    // Let try to connect to the server
    let client = connect("localhost", config.listen_port, false).await;
    // Note that connect already sends a handshake message
    assert!(client.is_ok());

    client.unwrap().shutdown().await.unwrap();

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
async fn test_launch_handshake_timeout() {
    let (server, config) = create_server().await;

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

    match server.await {
        Ok(_) => (),
        Err(e) => {
            // Should be a cancel error
            assert_eq!(e.is_cancelled(), true);
        }
    }
}
