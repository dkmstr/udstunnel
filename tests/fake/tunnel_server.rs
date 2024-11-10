extern crate udstunnel;

use mockall;
use mockall::mock;
use tokio::{self, task::JoinHandle};
use udstunnel::tunnel::udsapi::UDSApiProvider;
use udstunnel::tunnel::{config, server, udsapi};

// Mock the UDSApiProvider trait
#[derive(Debug)]
pub struct UDSApiProviderMock;

#[async_trait::async_trait]
impl UDSApiProvider for UDSApiProviderMock {
    async fn request(
        &self,
        _ticket: &str,
        _message: &str,
        _query_params: Option<&str>,
    ) -> Result<udsapi::UdsTicketResponse, std::io::Error> {
        Ok(udsapi::UdsTicketResponse {
            host: "localhost".to_string(),
            port: 9999,
            notify: "notify_012345678901234567890123456789012".to_string(),
        })
    }
}

#[allow(dead_code)] // For some reason, thinks that this function is not used (maybe because it's used in tests only)
pub async fn create(config: &config::Config) -> JoinHandle<()> {
    let launch_config = config.clone();
    let server = tokio::spawn(async move {
        let server = server::TunnelServer::new(&launch_config)
            .with_provider(std::sync::Arc::new(UDSApiProviderMock {}));
        let result = server.run().await;
        assert!(result.is_ok());
    });

    // Should be listening on configure port, let's wait a bit to
    // allow tokio to start the server
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    server
}
