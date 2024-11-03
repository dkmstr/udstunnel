extern crate udstunnel;

use tokio::{self, io::AsyncWriteExt};

use env_logger::Env;

//#[cfg(test)]
//use mockall::automock;

//#[cfg_attr(test, automock)]


use udstunnel::tunnel::{server::launch, client::connect};

#[tokio::test]
async fn test_launch() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let server = tokio::spawn(async {
        let result = launch().await;
        assert!(result.is_ok());
    });

    let client = connect("localhost", 4443, false).await;
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