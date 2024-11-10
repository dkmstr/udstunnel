extern crate udstunnel;

use tokio::{self, task::JoinHandle};

use udstunnel::tunnel::{config, server};

#[allow(dead_code)] // For some reason, thinks that this function is not used (maybe because it's used in tests only)
pub async fn create(config: &config::Config) -> JoinHandle<()> {
    let launch_config = config.clone();
    let server = tokio::spawn(async move {
        let server = server::TunnelServer::new(&launch_config);
        let result = server.run().await;
        assert!(result.is_ok());
    });

    // Should be listening on configure port, let's wait a bit to
    // allow tokio to start the server
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    server
}
