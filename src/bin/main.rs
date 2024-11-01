use tokio::{self, io};
use env_logger::Env;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("Hello!!");
    Ok(())
}
