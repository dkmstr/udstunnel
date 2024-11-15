use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use reqwest::ClientBuilder;

use super::{config, consts};

#[derive(Serialize, Deserialize, Debug)]
pub struct UdsTicketResponse {
    pub host: String,
    pub port: u16,
    pub notify: String,
}

impl Default for UdsTicketResponse {
    fn default() -> Self {
        UdsTicketResponse {
            host: String::new(),
            port: 0,
            notify: String::new(),
        }
    }
}

#[async_trait]
pub trait UDSApiProvider: Send + Sync {
    async fn request(
        &self,
        ticket: &str,
        message: &str,
        query_params: Option<&str>,
    ) -> Result<UdsTicketResponse, std::io::Error>;

    async fn get_ticket(
        &self,
        ticket: &str,
        ip: &str,
    ) -> Result<UdsTicketResponse, std::io::Error> {
        self.request(ticket, &ip, None).await
    }

    async fn notify_end(
        &self,
        ticket: &str,
        sent: u64,
        recv: u64,
        duration: std::time::Duration,
    ) -> Result<UdsTicketResponse, std::io::Error> {
        // Ignore response
        let _ = self
            .request(
                ticket,
                "stop",
                Some(
                    format!("sent={}&recv={}&elapsed={}", sent, recv, duration.as_secs()).as_str(),
                ),
            )
            .await;
        // Return empty response
        Ok(UdsTicketResponse {
            host: String::new(),
            port: 0,
            notify: String::new(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct HttpUDSApiProvider {
    pub verify_ssl: bool,
    pub timeout: std::time::Duration,
    pub server: String,
    pub token: String,
}

impl HttpUDSApiProvider {
    pub fn new(config: &config::Config) -> Self {
        HttpUDSApiProvider {
            verify_ssl: config.uds_verify_ssl,
            timeout: config.uds_timeout,
            server: config.uds_server.clone(),
            token: config.uds_token.clone(),
        }
    }
}

#[async_trait]
impl UDSApiProvider for HttpUDSApiProvider {
    async fn request(
        &self,
        ticket: &str,
        message: &str,
        query_params: Option<&str>,
    ) -> Result<UdsTicketResponse, std::io::Error> {
        // 1.- Try to get the ticket from UDS Server
        // 2.- If ticket is not found, log the error and return (caller will close the connection)
        // 3.- If ticket is found, we will receive (json):
        // { 'host': '....', 'port': '....', 'notify': '....' }
        // Where host it te host to connect, port is the port to connect and notify is the UDS ticket used to notification

        let client = match ClientBuilder::new()
            .use_rustls_tls()
            .danger_accept_invalid_certs(self.verify_ssl)
            .read_timeout(self.timeout)
            .connect_timeout(self.timeout)
            .user_agent(consts::USER_AGENT)
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                log::error!("Error creating UDS client: {:?}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ));
            }
        };

        let query = if let Some(query) = query_params {
            // If message already contains ?, append & instead of ?
            format!("{}{}", if message.contains('?') { "&" } else { "?" }, query)
        } else {
            String::new()
        };

        let url = format!(
            "{}/{}/{}/{}{}",
            self.server, ticket, message, self.token, query
        );

        let response = match client.get(&url).timeout(self.timeout).send().await {
            Ok(response) => response,
            Err(e) => {
                log::error!("Error requesting UDS: {:?}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ));
            }
        };

        // Extract json if response is fine
        if response.status().is_success() {
            let uds_response: UdsTicketResponse = response.json().await.unwrap_or_default();
            log::debug!("UDS Response: {:?}", uds_response);
            return Ok(uds_response);
        } else {
            log::error!("UDS Response status error: {:?}", response);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("UDS Response status error: {:?}", response),
            ));
        }
    }
}
