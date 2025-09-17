use std::{sync::Arc, time::Duration};

use log::{debug, info};

use udstunnel::tunnel::{self, config, consts, event, server, stats};

#[cfg(unix)]
use tokio::signal::unix::{signal as unix_signal, SignalKind};

use tokio::{
    select,
    signal::{self},
    time::timeout,
};

#[tokio::main(flavor = "multi_thread")]
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

    let stats = Arc::new(stats::Stats::new());

    if tunnel {
        let tunnel = server::TunnelServer::new(&config, stats.clone());

        let stop_event = event::Event::new();

        let ctrl_c = signal::ctrl_c();
        #[cfg(unix)]
        let mut terminate = unix_signal(SignalKind::terminate())?;

        let task_stopper = stop_event.clone();
        let tunnel_task = tokio::spawn(async move {
            if let Err(e) = tunnel.run(task_stopper).await {
                info!("Tunnel server error: {:?}", e);
            }
        });

        #[cfg(unix)]
        select! {
            _ = ctrl_c => {
                info!("Ctrl-C received, stopping tunnel server");
                stop_event.set().unwrap();
            }
            _ = terminate.recv() => {
                info!("SIGTERM received, stopping tunnel server");
                stop_event.set().unwrap();
            }
        }
        #[cfg(not(unix))]
        select! {
            _ = ctrl_c => {
                info!("Ctrl-C received, stopping tunnel server");
                stop_event.set().unwrap();
            }
        }

        // Ensure tunnel task is finished
        // While stats get_concurrent_connections is not 0, we wait (with a timeout)
        info!("Waiting for tunnel relay tasks to finish");
        timeout(Duration::from_secs(8), async {
            while stats.get_concurrent_connections() > 0 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
        .await
        .expect("Timeout waiting for concurrent connections to be 0. Force stopping");

        info!("Waiting for tunnel task to finish");
        timeout(Duration::from_secs(8), tunnel_task)
            .await
            .expect("Tunnel task should never fail")
            .expect("Timeout waiting for tunnel task. Force stopping");
    } else {
        // TODO: Implement stats
    }

    Ok(())

    // let _ = launch().await?;

    // println!("Hello!!");
    // Ok(())
}
