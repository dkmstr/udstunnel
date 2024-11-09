extern crate udstunnel;
use tokio::net::TcpListener;

use udstunnel::{config, tunnel};

pub async fn read() -> config::Config {
    let mut config = config::ConfigLoader::new()
        .with_filename("tests/udstunnel.conf")
        .load()
        .unwrap();

    // Get a free por for the configuration, so we can run multiple tests
    match TcpListener::bind(format!("{}:0", config.listen_address)).await {
        Ok(listener) => {
            let addr = listener.local_addr().unwrap();
            config.listen_port = addr.port();
        }
        Err(e) => {
            panic!("Error binding listener: {:?}", e);
        }
    }

    tunnel::log::setup(&None, &config.loglevel);
    config
}
