use std::time::Duration;

/// The `ConfigLoader` struct is responsible for loading and managing the configuration
/// for the UDS tunnel application. It provides methods to set various configuration
/// parameters and load the configuration from a file or environment variables.
///
/// # Fields
///
/// - `filename`: The name of the configuration file to load.
/// - `uds_server`: Optional UDS server location.
/// - `uds_token`: Optional UDS token to use.
///
/// # Methods
///
/// - `new() -> Self`: Creates a new `ConfigLoader` object with default configuration file paths.
/// - `with_filename(&mut self, file: String) -> &mut Self`: Sets the configuration file to load.
/// - `with_uds_server(&mut self, server: String) -> &mut Self`: Sets the UDS server location.
/// - `with_uds_token(&mut self, token: String) -> &mut Self`: Sets the UDS token to use.
/// - `load(&self) -> Result<types::Config, config::ConfigError>`: Loads the configuration from the specified file and environment variables, and returns a `Config` object.
///
/// # Configuration Loading Order
///
/// The configuration is loaded in the following order:
/// 1. Default values are set.
/// 2. Configuration file is loaded (if provided).
/// 3. Environment variables with the prefix `udstunnel` are loaded.
///
/// # Example
///
/// ```rust
/// use udstunnel::config::ConfigLoader;
/// let config = ConfigLoader::new()
///     .with_filename("custom_config.conf".into())
///     .with_uds_server("https://example.com/uds".into())
///     .with_uds_token("example_token".into())
///     .load()
///     .unwrap();
/// ```

#[derive(Debug, Clone)]
pub struct Config {
    pub pidfile: String,
    pub user: String,

    pub loglevel: String,
    pub logfile: Option<String>,
    pub logsize: u32,
    pub lognumber: u32,

    pub listen_address: String,
    pub listen_port: u16,

    pub ipv6: bool,

    pub workers: u8,

    pub ssl_min_tls_version: String, // Valid values are 1.2, 1.3 (1.0 and 1.1 are not supported)
    pub ssl_certificate: String,
    pub ssl_certificate_key: String,
    //pub ssl_password: String,  // Maybe supported in the future
    pub ssl_ciphers: String,

    pub uds_server: String,
    pub uds_token: String,
    pub uds_timeout: Duration,
    pub uds_verify_ssl: bool,

    pub command_timeout: Duration,

    pub secret: String,
    pub allow: Vec<String>,
    // Not used on rust
    // use_uvloop: bool
}


pub struct ConfigLoader {
    filename: String,
    uds_server: Option<String>,
    uds_token: Option<String>,
}

impl ConfigLoader {
    /// Create a new ConfigLoader object
    pub fn new() -> Self {
        let config_file: String;
        if cfg!(debug_assertions) {
            config_file = "udstunnel.conf".into();
        } else {
            config_file = "/etc/udstunnel.conf".into();
        }
        ConfigLoader {
            filename: config_file.into(),
            uds_server: None,
            uds_token: None,
        }
    }

    /// Set the configuration file to load
    pub fn with_filename(&mut self, file: &str) -> &mut Self {
        self.filename = file.to_string();
        self
    }

    /// Set the UDS server location (https://...)
    pub fn with_uds_server(&mut self, server: &str) -> &mut Self {
        self.uds_server = Some(server.to_string());
        self
    }

    /// Set the UDS token to use
    pub fn with_uds_token(&mut self, token: &str) -> &mut Self {
        self.uds_token = Some(token.to_string());
        self
    }

    /// The `load` method in the `ConfigLoader` struct is responsible for loading the configuration
    /// settings from various sources such as configuration files and environment variables. Here's a
    /// breakdown of what the method does:
    /// 1. Set default values for the configuration settings.
    /// 2. Load the configuration file specified by the user.
    /// 3. Load environment variables with the prefix `udstunnel`, overriding any existing values on the configuration file.
    /// 4. Return a `Result` containing the loaded configuration settings.
    pub fn load(&self) -> Result<Config, config::ConfigError> {
        // The order of the configuration search is:
        //   * /etc/udstunnel.conf if not DEBUG
        //   * udstunnel.conf in the current directory if DEBUG
        //   * Panic if no configuration is found
        // If an override file is provided, use it
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let cfg_reader = config::Config::builder()
            .set_default("pidfile", "/var/run/udstunnel.pid")?
            .set_default("user", "nobody")?
            .set_default("loglevel", "INFO")?
            .set_default("logfile", "")?
            .set_default("logsize", "10M")?
            .set_default("lognumber", 4)?
            .set_default("address", "0.0.0.0")?
            .set_default("port", 4443)?
            .set_default("ipv6", false)?
            .set_default("workers", num_cores as u8)?
            .set_default("ssl_min_tls_version", "1.2")?
            .set_default("ssl_certificate", "/etc/certs/server.pem")?
            .set_default("ssl_certificate_key", "/etc/certs/key.pem")?
            //.set_default("ssl_password", "")?
            .set_default("ssl_ciphers", "")?
            .set_default(
                "uds_server",
                if let Some(uds_server) = self.uds_server.clone() {
                    uds_server
                } else {
                    "".to_string()
                },
            )?
            .set_default(
                "uds_token",
                if let Some(uds_token) = self.uds_token.clone() {
                    uds_token
                } else {
                    "".to_string()
                },
            )?
            .set_default("uds_timeout", 10.0)?
            .set_default("uds_verify_ssl", true)?
            .set_default("command_timeout", 3.0)?
            .set_default("secret", "")?
            .set_default("allow", "")?
            .add_source(config::File::new(&self.filename, config::FileFormat::Ini).required(false))
            .add_source(config::Environment::with_prefix("udstunnel"))
            .build()?;

        // Get log size in bytes. Allowed prefixes are K, M and G
        let logsize = cfg_reader.get::<String>("logsize")?;
        let multiplier = match logsize.chars().last() {
            Some('K') => 1024,
            Some('M') => 1024 * 1024,
            Some('G') => 1024 * 1024 * 1024,
            _ => 1,
        };
        let logsize = std::cmp::max(
            multiplier
                * logsize[..logsize.len() - 1]
                    .parse::<u32>()
                    .unwrap_or_default(),
            1024 * 1024,
        );

        // Allow is a comma separated list of IP addresses
        let allow = cfg_reader.get::<String>("allow").unwrap_or_default();
        let allow = allow
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let logfile = cfg_reader.get::<String>("logfile")?;
        let logfile = if logfile.is_empty() {
            None
        } else {
            Some(logfile)
        };
        let command_timeout = cfg_reader
            .get::<f32>("command_timeout")
            .unwrap_or(3.0)
            .min(16.0)
            .max(0.4);
        let command_timeout = Duration::from_millis((command_timeout * 1000.0) as u64);
        let uds_timeout = cfg_reader
            .get::<f32>("uds_timeout")
            .unwrap_or(10.0)
            .min(60.0)
            .max(0.1);
        let uds_timeout = Duration::from_millis((uds_timeout * 1000.0) as u64);

        // Crate a configuration object
        Ok(Config {
            pidfile: cfg_reader.get("pidfile")?,
            user: cfg_reader.get("user")?,
            loglevel: cfg_reader.get::<String>("loglevel")?.to_uppercase(),
            logfile,
            logsize,
            lognumber: cfg_reader.get("lognumber")?,
            listen_address: cfg_reader.get("address")?,
            listen_port: cfg_reader.get("port")?,
            ipv6: cfg_reader.get("ipv6")?,
            workers: cfg_reader.get("workers")?,
            ssl_min_tls_version: cfg_reader.get("ssl_min_tls_version")?,
            ssl_certificate: cfg_reader.get("ssl_certificate")?,
            ssl_certificate_key: cfg_reader.get("ssl_certificate_key")?,
            //ssl_password: cfg_reader.get("ssl_password")?,
            ssl_ciphers: cfg_reader.get("ssl_ciphers")?,
            uds_server: cfg_reader.get("uds_server")?,
            uds_token: cfg_reader.get("uds_token")?,
            uds_timeout,
            uds_verify_ssl: cfg_reader.get("uds_verify_ssl")?,
            command_timeout,
            secret: cfg_reader.get("secret")?,
            allow,
        })
    }
}
