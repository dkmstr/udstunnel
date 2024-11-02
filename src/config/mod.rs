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
/// let config = ConfigLoader::new()
///     .with_filename("custom_config.conf".into())
///     .with_uds_server("https://example.com/uds".into())
///     .with_uds_token("example_token".into())
///     .load()
///     .unwrap();
/// ```
pub mod types;

use config;

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
    pub fn with_filename(&mut self, file: String) -> &mut Self {
        self.filename = file;
        self
    }

    /// Set the UDS server location (https://...)
    pub fn with_uds_server(&mut self, server: String) -> &mut Self {
        self.uds_server = Some(server);
        self
    }

    /// Set the UDS token to use
    pub fn with_uds_token(&mut self, token: String) -> &mut Self {
        self.uds_token = Some(token);
        self
    }

    /// The `load` method in the `ConfigLoader` struct is responsible for loading the configuration
    /// settings from various sources such as configuration files and environment variables. Here's a
    /// breakdown of what the method does:
    /// 1. Set default values for the configuration settings.
    /// 2. Load the configuration file specified by the user.
    /// 3. Load environment variables with the prefix `udstunnel`, overriding any existing values on the configuration file.
    /// 4. Return a `Result` containing the loaded configuration settings.
    pub fn load(&self) -> Result<types::Config, config::ConfigError> {
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
            .set_default("logfile", "/var/log/udstunnel.log")?
            .set_default("logsize", "10M")?
            .set_default("lognumber", 4)?
            .set_default("address", "[::]")?
            .set_default("port", 4443)?
            .set_default("ipv6", false)?
            .set_default("workers", num_cores as u8)?
            .set_default("ssl_min_tls_version", "1.2")?
            .set_default("ssl_certificate", "/etc/certs/server.pem")?
            .set_default("ssl_certificate_key", "/etc/certs/key.pem")?
            .set_default("ssl_password", "")?
            .set_default("ssl_ciphers", "")?
            .set_default("ssl_dhparam", "")?
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

        let _sslcert = cfg_reader.get::<String>("ssl_certificate")?;

        // Crate a configuration object
        Ok(types::Config {
            pidfile: cfg_reader.get("pidfile")?,
            user: cfg_reader.get("user")?,
            loglevel: cfg_reader.get::<String>("loglevel")?.to_uppercase(),
            logfile: cfg_reader.get("logfile")?,
            logsize,
            lognumber: cfg_reader.get("lognumber")?,
            listen_address: cfg_reader.get("address")?,
            listen_port: cfg_reader.get("port")?,
            ipv6: cfg_reader.get("ipv6")?,
            workers: cfg_reader.get("workers")?,
            ssl_min_tls_version: cfg_reader.get("ssl_min_tls_version")?,
            ssl_certificate: cfg_reader.get("ssl_certificate")?,
            ssl_certificate_key: cfg_reader.get("ssl_certificate_key")?,
            ssl_password: cfg_reader.get("ssl_password")?,
            ssl_ciphers: cfg_reader.get("ssl_ciphers")?,
            ssl_dhparam: cfg_reader.get("ssl_dhparam")?,
            uds_server: cfg_reader.get("uds_server")?,
            uds_token: cfg_reader.get("uds_token")?,
            uds_timeout: cfg_reader.get("uds_timeout")?,
            uds_verify_ssl: cfg_reader.get("uds_verify_ssl")?,
            command_timeout: cfg_reader.get("command_timeout")?,
            secret: cfg_reader.get("secret")?,
            allow,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_default() {
        let config = ConfigLoader::new()
            .with_filename("non_existing_for_tests".into())
            .load()
            .unwrap();
        assert_eq!(config.pidfile, "/var/run/udstunnel.pid");
        assert_eq!(config.user, "nobody");
        assert_eq!(config.loglevel, "INFO");
        assert_eq!(config.logfile, "/var/log/udstunnel.log");
        assert_eq!(config.logsize, 10 * 1024 * 1024);
        assert_eq!(config.lognumber, 4);
        assert_eq!(config.listen_address, "[::]");
        assert_eq!(config.listen_port, 4443);
        assert_eq!(config.ipv6, false);
        assert_eq!(config.workers > 0, true);
        assert_eq!(config.ssl_min_tls_version, "1.2");
        assert_eq!(config.ssl_certificate, "/etc/certs/server.pem");
        assert_eq!(config.ssl_certificate_key, "/etc/certs/key.pem");
        assert_eq!(config.ssl_password, "");
        assert_eq!(config.ssl_ciphers, "");
        assert_eq!(config.ssl_dhparam, "");
        assert_eq!(config.uds_server, "");
        assert_eq!(config.uds_token, "");
        assert_eq!(config.uds_timeout, 10.0);
        assert_eq!(config.uds_verify_ssl, true);
        assert_eq!(config.command_timeout, 3.0);
        assert_eq!(config.secret, "");
        assert_eq!(config.allow, Vec::<String>::new());
    }

    #[test]
    fn test_load_config_from_file() {
        let config = ConfigLoader::new()
            .with_filename("tests/udstunnel.conf".into())
            .load()
            .unwrap();
        assert_eq!(config.pidfile, "/tmp/udstunnel.pid");
        assert_eq!(config.user, "dkmaster");
        assert_eq!(config.loglevel, "DEBUG");
        assert_eq!(config.logfile, "/tmp/tunnel.log");
        assert_eq!(config.logsize, 120 * 1024 * 1024);
        assert_eq!(config.lognumber, 3);
        assert_eq!(config.listen_address, "0.0.0.0");
        assert_eq!(config.listen_port, 7777);
        assert_eq!(config.ipv6, true);
        assert_eq!(config.workers > 0, true);
        assert_eq!(config.ssl_min_tls_version, "1.3");
        assert_eq!(config.ssl_certificate, "/tmp/server.pem");
        assert_eq!(config.ssl_certificate_key, "/tmp/key.pem");
        assert_eq!(config.ssl_password, "MyPassword");
        assert_eq!(
            config.ssl_ciphers,
            "ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512"
        );
        assert_eq!(config.ssl_dhparam, "/tmp/dhparam.pem");
        assert_eq!(config.uds_server, "https://127.0.0.1:8000/uds/rest/tunnel/ticket");
        assert_eq!(config.uds_token, "uds_token");
        assert_eq!(config.uds_timeout, 16.0);
        assert_eq!(config.uds_verify_ssl, false);
        assert_eq!(config.command_timeout, 23.0);
        assert_eq!(config.secret, "MySecret");
        assert_eq!(config.allow, vec!["127.0.0.1", "127.0.0.2"]);
    }
}
