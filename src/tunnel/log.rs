use std::io::Write;

use env_logger;

// Setup logging
pub fn setup(filename: &Option<String>, level: &str) {
    // If already initialized, return
    let mut target = env_logger::Target::Stderr;
    if let Some(logfile) = filename {
        target = env_logger::Target::Pipe(Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logfile)
                .unwrap(),
        ));
    }

    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level.to_string()))
        .target(target)
        .format_module_path(false)
        .format_timestamp_millis()
        .format(|buf, record| writeln!(buf, "{} - {}", record.level(), record.args()))
        .init();
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level.to_string()))
        .target(target)
        .format_module_path(false)
        .format_timestamp_millis()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} - {} {} {}",
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .try_init().unwrap_or_default();

    // let _ = cfg_builder
    //     .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
    //     .format_module_path(true);
    // #[cfg(not(debug_assertions))]
}
