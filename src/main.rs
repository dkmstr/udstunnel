use log::{debug, info};

use udstunnel::tunnel::{self, consts};

use udstunnel::tunnel::{server, config};

use clap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = clap::Command::new("udstunnel")
        .version("5.0.0")
        .author("Adolfo Gómez <dkmaster@dkmon.com>")
        .about("UDP Tunnel Server")
        // In fact, if no stats and no detailed-stats, we should start the tunnel...
        .arg(
            clap::Arg::new("config")
                .short('c')
                .long("config")
                .help("Config file to use")
                .action(clap::ArgAction::Set),
        )
        .arg(
            clap::Arg::new("tunnel")
                .short('t')
                .long("tunnel")
                .help("Starts the tunnel server. (default, backwards compatible)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("stats")
                .short('s')
                .long("stats")
                .help("get current global stats from RUNNING tunnel")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("detailed-stats")
                .short('d')
                .long("detailed-stats")
                .help("get current detailed stats from RUNNING tunnel")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("ipv6")
                .short('6')
                .long("ipv6")
                .help("Force IPv6 for tunnel server")
                .action(clap::ArgAction::SetTrue),
        );

    let matches = cmd.get_matches();
    let stats = matches.get_flag("stats");
    let detailed_stats = matches.get_flag("detailed-stats");
    let tunnel = matches.get_flag("tunnel") || (!stats && !detailed_stats);
    let config_file = matches.try_get_one::<String>("config").unwrap();

    let config = config::ConfigLoader::new()
        .with_filename(if let Some(config_file) = config_file {
            config_file
        } else {
            consts::CONFIGFILE
        })
        .load()
        .unwrap();

    tunnel::log::setup(&config.logfile, &config.loglevel);

    info!("Starting udstunnel v{}", consts::VERSION);

    debug!("Config: {:?}", config);
    //println!("{}", cmd.render_long_help());

    if tunnel {
        let tunnel = server::TunnelServer::new(&config);

        match tunnel.run().await {
            Ok(_) => {
                info!("Tunnel server started");
            }
            Err(e) => {
                info!("Error starting tunnel server: {:?}", e);
            }
        }
    }

    Ok(())

    // let _ = launch().await?;

    // println!("Hello!!");
    // Ok(())
}
