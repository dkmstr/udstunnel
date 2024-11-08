#[cfg(test)]
extern crate udstunnel;

use tokio::{
    self,
    io::AsyncWriteExt,
    net::TcpStream,
    task::JoinHandle,
    time::{timeout, Duration},
};

use env_logger::Env;

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]

use udstunnel::{
    config,
    tunnel::{self, client::connect, server::launch},
};

fn get_config() -> config::Config {
    let config = config::ConfigLoader::new()
        .with_filename("tests/udstunnel.conf")
        .load()
        .unwrap();

    tunnel::log::setup(&None, &config.loglevel);
    config
}

async fn create_server() -> (JoinHandle<()>, config::Config) {
    let config = get_config();

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
