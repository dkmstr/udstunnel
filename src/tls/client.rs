use tokio::{io, net::TcpStream};

use std::fmt;
use std::sync::Arc;

use log::debug;

use tokio_rustls::{
    client::TlsStream,
    rustls::{
        pki_types::{pem::PemObject, CertificateDer},
        RootCertStore,
    },
    TlsConnector,
};

use super::callbacks::TLSClientCallback;
use super::noverify::NoVerifySsl;

pub struct ConnectionBuilder {
    server: String,
    port: u16,
    verify: bool,
    connect_callback: Option<Box<dyn TLSClientCallback>>,
    certificate_path: Option<String>,
}

impl fmt::Debug for ConnectionBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionBuilder")
            .field("server", &self.server)
            .field("port", &self.port)
            .field("verify", &self.verify)
            .finish()
    }
}

impl ConnectionBuilder {
    pub fn new(server: &str, port: u16) -> Self {
        ConnectionBuilder {
            server: String::from(server),
            port,
            verify: true,
            connect_callback: None,
            certificate_path: None,
        }
    }

    pub fn with_verify_ssl(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_connect_callback<T: TLSClientCallback + 'static>(mut self, callback: T) -> Self {
        self.connect_callback = Some(Box::new(callback));
        self
    }

    pub fn with_certificate_path(mut self, path: &str) -> Self {
        self.certificate_path = Some(String::from(path));
        self
    }

    pub async fn connect(self) -> io::Result<TlsStream<TcpStream>> {
        debug!("Connecting to {}:{}", self.server, self.port);
        // Load RootCertStore from /etc/ssl/certs/ca-certificates.crt
        let cert_path = self
            .certificate_path
            .unwrap_or("/etc/ssl/certs/ca-certificates.crt".to_string());
        debug!("Loading certificates from: {}", cert_path);

        let certs: Vec<CertificateDer> = CertificateDer::pem_file_iter(cert_path)
            .unwrap()
            .map(|cert| cert.unwrap())
            .collect();

        let mut root_store = RootCertStore::empty();
        root_store.add_parsable_certificates(certs);

        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        if !self.verify {
            config
                .dangerous()
                .set_certificate_verifier(NoVerifySsl::new());
        }

        let connector = TlsConnector::from(Arc::new(config));
        let server_name = self.server.clone().try_into().unwrap();
        let mut stream = TcpStream::connect(format!("{}:{}", self.server, self.port))
            .await
            .unwrap();

        if let Some(connect_callback) = self.connect_callback {
            connect_callback.process(&mut stream).await?;
        }

        let tls_stream: tokio_rustls::client::TlsStream<TcpStream> =
            connector.connect(server_name, stream).await.unwrap();

        Ok(tls_stream)
    }
}
