extern crate udstunnel;

use std::sync::{Arc, Mutex};

use log;
use tokio::{self, task::JoinHandle};

use udstunnel::tunnel::udsapi::UDSApiProvider;
use udstunnel::tunnel::{config, server, udsapi};

#[derive(Debug)]
pub struct Request {
    pub ticket: String,
    pub message: String,
    pub query_params: Option<String>,
}

// Mock the UDSApiProvider trait
#[derive(Debug)]
pub struct UDSApiProviderMock {
    req: Arc<Mutex<Vec<Request>>>,
}

impl UDSApiProviderMock {
    pub fn new() -> Self {
        UDSApiProviderMock {
            req: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl UDSApiProvider for UDSApiProviderMock {
    async fn request(
        &self,
        ticket: &str,
        message: &str,
        query_params: Option<&str>,
    ) -> Result<udsapi::UdsTicketResponse, std::io::Error> {
        self.req.lock().unwrap().push(Request {
            ticket: ticket.to_string(),
            message: message.to_string(),
            query_params: query_params.map(|s| s.to_string()),
        });

        log::debug!(
            "Mocked request: ticket: {} msg: {} param:{}",
            ticket,
            message,
            query_params.unwrap_or("")
        );

        Ok(udsapi::UdsTicketResponse {
            host: "localhost".to_string(),
            port: 9999,
            notify: "notify_012345678901234567890123456789012".to_string(),
        })
    }
}

/// Creates and starts a tunnel server.
///
/// This function creates a tunnel server using the provided configuration.
/// If `override_provider` is `true`, it uses a mock `UDSApiProvider` for testing purposes.
/// Otherwise, it uses the default `HttpUDSApiProvider`.
///
/// # Arguments
///
/// * `config` - A reference to the configuration for the tunnel server.
/// * `override_provider` - A boolean indicating whether to use the mock `UDSApiProvider`.
///
/// # Returns
///
/// A tuple containing:
/// * A `JoinHandle` for the spawned server task.
/// * An optional `Arc<Mutex<Vec<String>>>` containing the request log if the mock provider is used.
#[allow(dead_code)]
pub async fn create(
    config: &config::Config,
    override_provider: bool,
) -> (JoinHandle<()>, Option<Arc<Mutex<Vec<Request>>>>) {
    let launch_config = config.clone();
    let provider: Arc<dyn UDSApiProvider>;
    let req;
    if override_provider {
        let mock = UDSApiProviderMock::new();
        req = Some(mock.req.clone());
        provider = Arc::new(mock);
    } else {
        provider = Arc::new(udsapi::HttpUDSApiProvider::new(&launch_config));
        req = None;
    }
    let server = tokio::spawn(async move {
        let mut tunnel = server::TunnelServer::new(&launch_config);
        if override_provider {
            tunnel = tunnel.with_provider(provider);
        }
        let result = tunnel.run().await;
        assert!(result.is_ok());
    });

    // Should be listening on configure port, let's wait a bit to
    // allow tokio to start the server
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    (server, req)
}
