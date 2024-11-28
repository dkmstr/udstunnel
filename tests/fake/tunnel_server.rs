extern crate udstunnel;

use std::sync::{Arc, Mutex};

use log;
use tokio::{self, task::JoinHandle};

use udstunnel::tunnel::udsapi::UDSApiProvider;
use udstunnel::tunnel::{config, event, server, stats, udsapi};

use super::remote::Remote;

#[allow(dead_code)]
pub struct Request {
    pub ticket: String,
    pub message: String,
    pub query_params: Option<String>,
}

// Mock the UDSApiProvider trait
pub struct UDSApiProviderMock {
    port: u16,
    req: Arc<Mutex<Vec<Request>>>,
}

impl UDSApiProviderMock {
    pub fn new(port: u16) -> Self {
        UDSApiProviderMock {
            port,
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
            host: "[::1]".to_string(),
            port: self.port,
            notify: "notify_012345678901234567890123456789012".to_string(),
        })
    }
}

#[allow(dead_code)]
pub struct TunnelServer {
    pub requests: Option<Arc<Mutex<Vec<Request>>>>,
    pub server_handle: JoinHandle<()>,
    pub remote_handle: JoinHandle<()>,
    pub stopper: event::Event,
    pub stats: Arc<stats::Stats>,
}

#[allow(dead_code)]
impl TunnelServer {
    pub async fn create(config: &config::Config, mock_remotes: bool) -> TunnelServer {
        let launch_config = config.clone();
        let provider: Arc<dyn UDSApiProvider>;
        let req;
        let remote = Remote::new(None);
        let remote_handle = remote.spawn();

        if mock_remotes {
            // Crate a fake remote, and use it also on mock provider
            let mock = UDSApiProviderMock::new(remote.port);
            req = Some(mock.req.clone());
            provider = Arc::new(mock);
        } else {
            provider = Arc::new(udsapi::HttpUDSApiProvider::new(&launch_config));
            req = None;
        }
        let task_provider = provider.clone();
        let stopper = event::Event::new();
        let task_stopper = stopper.clone();
        let stats = Arc::new(stats::Stats::new());
        let stats_co = stats.clone();
        let server_handle = tokio::spawn(async move {
            let mut tunnel = server::TunnelServer::new(&launch_config, stats_co.clone());
            if mock_remotes {
                tunnel = tunnel.with_provider(task_provider);
            }
            let result = tunnel.run(task_stopper).await;
            if let Err(e) = result {
                log::error!("Error: {:?}", e);
                panic!("Error: {:?}", e);
            }
        });

        // Should be listening on configure port, let's wait a bit to
        // allow tokio to start the server
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        TunnelServer {
            requests: req,
            remote_handle,
            server_handle,
            stopper,
            stats,
        }
    }

    pub fn abort(&self) {
        self.stopper.set().unwrap();
        // The remote need to be aborted too, but using abort, it's for testing
        self.remote_handle.abort();
    }
}
