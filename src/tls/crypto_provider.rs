use rustls::{
    crypto::{aws_lc_rs, CryptoProvider},
    SupportedCipherSuite,
};

use log;

fn openssl_to_rustls_cipher_name(cipher: &str) -> Option<SupportedCipherSuite> {
    let rust_cipher_name = match cipher {
        // TLS 1.3 Suites
        "TLS_AES_256_GCM_SHA384" => Some("TLS13_AES_256_GCM_SHA384"),
        "TLS_AES_128_GCM_SHA256" => Some("TLS13_AES_128_GCM_SHA256"),
        "TLS_CHACHA20_POLY1305_SHA256" => Some("TLS13_CHACHA20_POLY1305_SHA256"),

        // TLS 1.2 Suites
        "ECDHE-ECDSA-AES256-GCM-SHA384" => Some("TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384"),
        "ECDHE-ECDSA-AES128-GCM-SHA256" => Some("TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256"),
        "ECDHE-ECDSA-CHACHA20-POLY1305-SHA256" => {
            Some("TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256")
        }
        "ECDHE-RSA-AES256-GCM-SHA384" => Some("TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"),
        "ECDHE-RSA-AES128-GCM-SHA256" => Some("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"),
        "ECDHE-RSA-CHACHA20-POLY1305-SHA256" => Some("TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256"),

        // En caso de que no se encuentre el ciphersuite
        _ => None,
    };
    // Only return the rustls cipher name if it is in the list of rustls::crypto::aws_lc_rs::ALL_CIPHER_SUITES

    if rust_cipher_name.is_some() {
        for suite in rustls::crypto::aws_lc_rs::ALL_CIPHER_SUITES.iter() {
            if suite.suite().as_str() == rust_cipher_name {
                return Some(*suite);
            }
        }
    }

    None
}

fn filter_cipher_suites(ciphers: &str) -> Vec<SupportedCipherSuite> {
    ciphers
        .split(':')
        .collect::<Vec<&str>>()
        .iter()
        .filter_map(|cipher| openssl_to_rustls_cipher_name(cipher))
        .collect()
}

pub fn provider(list_of_ciphers: &str) -> CryptoProvider {
    let mut ciphers = filter_cipher_suites(list_of_ciphers);
    if ciphers.is_empty() {
        log::warn!(
            "No valid cipher suites found in {}, using default",
            list_of_ciphers
        );
        ciphers = rustls::crypto::aws_lc_rs::DEFAULT_CIPHER_SUITES.to_vec();
    }
    log::debug!("cipher_suites: {:?}", ciphers);

    rustls::crypto::CryptoProvider {
        cipher_suites: ciphers,
        ..aws_lc_rs::default_provider()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_empty_cipher_list() {
        let ciphers = "";
        let provider = provider(ciphers);
        assert_eq!(
            provider.cipher_suites.len(),
            rustls::crypto::aws_lc_rs::DEFAULT_CIPHER_SUITES.len()
        );
    }
    
    #[test]
    fn test_invalid_cipher_list() {
        let ciphers = "ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512";
        let provider = provider(ciphers);
        assert_eq!(
            provider.cipher_suites.len(),
            rustls::crypto::aws_lc_rs::DEFAULT_CIPHER_SUITES.len()
        );
    }
    
    #[test]
    fn test_some_valid_cipher_list() {
        let ciphers = "ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-CHACHA20-POLY1305-SHA256";
        let provider = provider(ciphers);
        assert_eq!(provider.cipher_suites.len(), 2);
    }
    
    #[test]
    fn test_valid_cipher_list() {
        let ciphers = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256";
        let provider = provider(ciphers);
        assert_eq!(provider.cipher_suites.len(), 3);
    }
}
