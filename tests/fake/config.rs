extern crate udstunnel;

use udstunnel::tunnel::{self, config};

use super::utils;

#[allow(dead_code)] // For some reason, thinks that this function is not used (maybe because it's used in tests only)
pub async fn read() -> config::Config {
    let mut config = config::ConfigLoader::new()
        .with_filename("tests/udstunnel.conf")
        .load()
        .unwrap();

    config.listen_port = utils::find_free_port(Some(&config.listen_address));

    tunnel::log::setup(&None, &config.loglevel);
    config
}
