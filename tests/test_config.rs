extern crate udstunnel;

use udstunnel::tunnel::config::ConfigLoader;
mod fake;

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
        assert_eq!(config.logfile, None);
        assert_eq!(config.logsize, 10 * 1024 * 1024);
        assert_eq!(config.lognumber, 4);
        assert_eq!(config.listen_address, "0.0.0.0");
        assert_eq!(config.listen_port, 4443);
        assert_eq!(config.ipv6, false);
        assert_eq!(config.workers > 0, true);
        assert_eq!(config.ssl_min_tls_version, "1.2");
        assert_eq!(config.ssl_certificate, "/etc/certs/server.pem");
        assert_eq!(config.ssl_certificate_key, "/etc/certs/key.pem");
        //assert_eq!(config.ssl_password, "");
        assert_eq!(config.ssl_ciphers, "");
        assert_eq!(config.uds_server, "");
        assert_eq!(config.uds_token, "");
        assert_eq!(config.uds_timeout, Duration::from_millis(10000));
        assert_eq!(config.uds_verify_ssl, true);
        assert_eq!(config.command_timeout, Duration::from_millis(3000));
        assert_eq!(config.secret, "");
        assert_eq!(config.allow, Vec::<String>::new());
    }

    #[test]
    fn test_load_config_from_file() {
        let config = ConfigLoader::new()
            .with_filename("tests/udstunnel.conf")
            .load()
            .unwrap();
        assert_eq!(config.pidfile, "/tmp/udstunnel.pid");
        assert_eq!(config.user, "dkmaster");
        assert_eq!(config.loglevel, "DEBUG");
        assert_eq!(config.logfile, Some("/tmp/tunnel.log".to_string()));
        assert_eq!(config.logsize, 120 * 1024 * 1024);
        assert_eq!(config.lognumber, 3);
        assert_eq!(config.listen_address, "[::]");
        assert_eq!(config.listen_port, 7777);
        assert_eq!(config.ipv6, true);
        assert_eq!(config.workers > 0, true);
        assert_eq!(config.ssl_min_tls_version, "1.3");
        assert_eq!(config.ssl_certificate, "tests/certs/cert.pem");
        assert_eq!(config.ssl_certificate_key, "tests/certs/key.pem");
        //assert_eq!(config.ssl_password, "MyPassword");
        assert_eq!(
            config.ssl_ciphers,
            "TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:ECDHE-ECDSA-CHACHA20-POLY1305-SHA256"
        );
        assert_eq!(config.uds_server, "http://127.0.0.1:8000/uds/rest/tunnel/ticket");
        assert_eq!(config.uds_token, "uds_token");
        assert_eq!(config.uds_timeout, Duration::from_millis(4000));
        assert_eq!(config.uds_verify_ssl, false);
        assert_eq!(config.command_timeout, Duration::from_millis(13000));
        assert_eq!(config.secret, "MySecret");
        assert_eq!(config.allow, vec!["127.0.0.1", "127.0.0.2"]);
    }
}