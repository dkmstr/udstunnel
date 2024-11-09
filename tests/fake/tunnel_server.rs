extern crate udstunnel;

use tokio::{self, task::JoinHandle};

use udstunnel::{config, tunnel::server::run};

pub async fn create(config: &config::Config) -> JoinHandle<()> {
    let launch_config = config.clone();
    let server = tokio::spawn(async move {
        let result = run(launch_config).await;
        assert!(result.is_ok());
    });

    // Should be listening on configure port, let's wait a bit to
    // allow tokio to start the server
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    server
}
