use serde::{Deserialize, Serialize};

use crate::config;

use reqwest::ClientBuilder;

use super::consts;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct UdsTicketResponse {
    pub host: String,
    pub port: u16,
    pub notify: String,
}

pub(crate) async fn request_from_uds(
    config: &config::Config,
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
        .danger_accept_invalid_certs(config.uds_verify_ssl)
        .read_timeout(config.uds_timeout)
        .connect_timeout(config.uds_timeout)
        .user_agent(consts::USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            log::error!("Error creating UDS client: {:?}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
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
        config.uds_server, ticket, message, config.uds_token, query
    );

    let response = client
        .get(&url)
        .timeout(config.uds_timeout)
        .send()
        .await
        .unwrap();

    // Extract json if response is fine
    if response.status().is_success() {
        let uds_response: UdsTicketResponse = response.json().await.unwrap();
        log::debug!("UDS Response: {:?}", uds_response);
        return Ok(uds_response);
    } else {
        log::error!("UDS Response: {:?}", response);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("UDS Response: {:?}", response),
        ));
    }
}
