use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

use crate::config;

use reqwest::ClientBuilder;

use super::consts;

pub(crate) async fn run(
    stream: &mut TlsStream<TcpStream>,
    ticket: String,
    uds_url: String,
    uds_verify_ssl: bool,
    uds_timeout: std::time::Duration,
) -> () {
    // 1.- Try to get the ticket from UDS Server
    // 2.- If ticket is not found, log the error and return (caller will close the connection)
    // 3.- If ticket is found, we will receive (json):
    // { 'host': '....', 'port': '....', 'notify': '....' }
    // Where host it te host to connect, port is the port to connect and notify is the UDS ticket used to notification

    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(uds_verify_ssl)
        .read_timeout(uds_timeout)
        .connect_timeout(uds_timeout)
        .user_agent(consts::USER_AGENT)
        .build()
        .unwrap();
}
