extern crate udstunnel;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};
use udstunnel::tunnel::config::Config;

const HOST: &str = "localhost";
const PORT: &str = "9999";
// 40 chars
const NOTIFY_TICKET: &str = "notify_012345678901234567890123456789012";

async fn broker_http_server(config: Config) -> () {
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

    // Create a listener. extract path for logging, but, do not matter what is
    // the path, we will always return 200 with this body:
    // { "host": HOST, "port": PORT, "ticket": NOTIFY_TICKET }
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    log::info!("Fake broker listening on {}", addr);

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 8192];
        let n = stream.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..n]);
        log::info!("Fake broker received: {}", request);

        let response = format!(
            "{{\"host\": \"{}\", \"port\": \"{}\", \"ticket\": \"{}\"}}",
            HOST, PORT, NOTIFY_TICKET
        );
        let content_length = response.len();
        let content_length = format!("Content-Length: {}\r\n", content_length);
        // Send the response
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n").await.unwrap();
        stream.write_all(content_length.as_bytes()).await.unwrap();
        stream.write_all(b"\r\n").await.unwrap();
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}

#[allow(dead_code)] // For some reason, thinks that this function is not used (maybe because it's used in tests only)
pub async fn create(config: &Config) -> JoinHandle<()> {
    tokio::spawn(broker_http_server(config.clone()))
}
