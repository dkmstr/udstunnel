use env_logger::Env;

use udstunnel::tunnel::server::launch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let _ = launch().await?;

    println!("Hello!!");
    Ok(())
}
