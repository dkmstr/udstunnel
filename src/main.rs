use env_logger::Env;
use log::{debug, info};

use udstunnel::tunnel::consts;

use udstunnel::{config, tunnel::server::launch};

use clap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // group.add_argument('-t', '--tunnel', help='Starts the tunnel server', action='store_true')
    // # group.add_argument('-r', '--rdp', help='RDP Tunnel for traffic accounting')
    // group.add_argument(
    //     '-s',
    //     '--stats',
    //     help='get current global stats from RUNNING tunnel',
    //     action='store_true',
    // )
    // group.add_argument(
    //     '-d',
    //     '--detailed-stats',
    //     help='get current detailed stats from RUNNING tunnel',
    //     action='store_true',
    // )
    // # Config file
    // parser.add_argument(
    //     '-c',
    //     '--config',
    //     help=f'Config file to use (default: {consts.CONFIGFILE})',
    //     default=consts.CONFIGFILE,
    // )
    // # If force ipv6
    // parser.add_argument(
    //     '-6',
    //     '--ipv6',
    //     help='Force IPv6 for tunnel server',
    //     action='store_true',
    // )
    // args = parser.parse_args()
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

    let mut target = env_logger::Target::Stderr;
    if let Some(logfile) = &config.logfile {
        target = env_logger::Target::Pipe(Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logfile)
                .unwrap(),
        ));
    }

    // Set loglevel
    env_logger::Builder::from_env(Env::default().default_filter_or(config.loglevel.clone()))
        .target(target)
        .init();

    info!("Starting udstunnel v{}", consts::VERSION);

    debug!("Config: {:?}", config);
    //println!("{}", cmd.render_long_help());

    Ok(())

    // let _ = launch().await?;

    // println!("Hello!!");
    // Ok(())
}
