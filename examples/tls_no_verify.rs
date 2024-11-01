use std::io::{stdout, Read, Write};
use std::cmp::min;
use std::net::TcpStream;
use std::sync::Arc;

use env_logger::Env;
use log::debug;


use tokio_rustls::{
    rustls::{
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        RootCertStore,
    },
};

use iotest::tls_noverify::NoVerifySsl;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Load RootCertStore from /etc/ssl/certs/ca-certificates.crt
    let certs: Vec<CertificateDer> = CertificateDer::pem_file_iter("/etc/ssl/certs/ca-certificates.crt")
        .unwrap()
        .map(|cert| cert.unwrap())
        .collect();

    let mut root_store = RootCertStore::empty();
    root_store.add_parsable_certificates(certs);

    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    config.dangerous().set_certificate_verifier(NoVerifySsl::new());


    // Allow using SSLKEYLOGFILE.
    config.key_log = Arc::new(rustls::KeyLogFile::new());

    let server_name = "db.dkmon.com".try_into().unwrap();
    let mut ssl_client_connection = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();
    let mut sock = TcpStream::connect("db.dkmon.com:443").unwrap();
    let mut tls = rustls::Stream::new(&mut ssl_client_connection, &mut sock);
    tls.write_all(
        concat!(
            "GET / HTTP/1.1\r\n",
            "Host: www.rust-lang.org\r\n",
            "Connection: close\r\n",
            "Accept-Encoding: identity\r\n",
            "\r\n"
        )
        .as_bytes(),
    )
    .unwrap();
    let ciphersuite = tls.conn.negotiated_cipher_suite().unwrap();
    debug!(
        "Current ciphersuite: {:?}",
        ciphersuite.suite()
    );
    let mut plaintext = Vec::new();
    tls.read_to_end(&mut plaintext).unwrap();
    debug!("Read {} bytes", plaintext.len());
    // First 512 bytes if available of the plaintext.
    let plaintext = &plaintext[..min(512, plaintext.len())];
    debug!("Plaintext: {:?}", String::from_utf8_lossy(plaintext));
}
