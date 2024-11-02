#[derive(Debug)]
pub struct Config {
    pub pidfile: String,
    pub user: String,

    pub loglevel: String,
    pub logfile: String,
    pub logsize: u32,
    pub lognumber: u32,

    pub listen_address: String,
    pub listen_port: u16,

    pub ipv6: bool,

    pub workers: u8,

    pub ssl_min_tls_version: String,  // Valid values are 1.2, 1.3 (1.0 and 1.1 are not supported)
    pub ssl_certificate: String,
    pub ssl_certificate_key: String,
    pub ssl_password: String,
    pub ssl_ciphers: String,
    pub ssl_dhparam: String,

    pub uds_server: String,
    pub uds_token: String,
    pub uds_timeout: f32,
    pub uds_verify_ssl: bool,

    pub command_timeout: f32,

    pub secret: String,
    pub allow: Vec<String>,

    // Not used on rust
    // use_uvloop: bool
}
