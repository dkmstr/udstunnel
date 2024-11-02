extern crate udstunnel;

use tokio::{self, io::AsyncWriteExt};
use udstunnel::tunnel::{server::launch, client::connect};

#[tokio::test]
async fn test_launch() {
    let server = tokio::spawn(async {
        let result = launch().await;
        assert!(result.is_ok());
    });

    let client = connect("localhost", 4443, false).await;
    assert!(client.is_ok());
    client.unwrap().shutdown().await.unwrap();

    server.aboyrt();

}
