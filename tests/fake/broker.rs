extern crate udstunnel;

use udstunnel::config::Config;

pub async fn broker_http_server(config: &Config) -> () {
    let host = config.uds_server.clone();
    // Should be http://host[:port]/uds/rest/tunnel/ticket for testing, we will not use ssl
    // Skip the http://
    if host.starts_with("https://") {
        panic!("This is a fake broker, it should not use https");
    }
    let host = host.replace("http://", "");
    // Split the host and port if present
    let mut host_port = host.split(':');
    let host = host_port.next().unwrap();
    let port = host_port.next().unwrap_or("80");

    log::info!("Fake broker running on {}:{}", host, port);

}