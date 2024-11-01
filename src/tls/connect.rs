use tokio::{io, net::TcpStream};

use std::{future::Future, sync::Arc};
use std::fmt;

use log::debug;

use tokio_rustls::{
    client::TlsStream,
    rustls::{
        pki_types::{pem::PemObject, CertificateDer},
        RootCertStore,
    },
    TlsConnector,
};

use super::noverify::NoVerifySsl;

// Type to represent a callback to be invoked
// when the connection is established but the tls is not yet negotiated.

pub struct ConnectionBuilder<F> where F: Future + Send + 'static {
    server: String,
    port: u16,
    verify: bool,
    callback: Option<F>,
}

impl<F> fmt::Debug for ConnectionBuilder<F> where F: Future + Send + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionBuilder")
            .field("server", &self.server)
            .field("port", &self.port)
            .field("verify", &self.verify)
            .finish()
    }
}

impl<F> ConnectionBuilder<F> where F: Future + Send + 'static {
    pub fn new(server: &str, port: u16) -> Self {
        ConnectionBuilder {
            server: String::from(server),
            port,
            verify: true,
            callback: None,
        }
    }

    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_callback(mut self, callback: F) -> Self {
        self.callback = Some(callback);
        self
    }

    pub async fn connect(self) -> io::Result<TlsStream<TcpStream>> {
        debug!("Connecting to {}:{}", self.server, self.port);
        // Load RootCertStore from /etc/ssl/certs/ca-certificates.crt
        let certs: Vec<CertificateDer> =
            CertificateDer::pem_file_iter("/etc/ssl/certs/ca-certificates.crt")
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
        let stream = TcpStream::connect(format!("{}:{}", self.server, self.port))
            .await
            .unwrap();
        // Invoke the callback if it is set.
        if let Some(callback) = self.callback {
            callback.await;
        }
        let tls_stream: tokio_rustls::client::TlsStream<TcpStream> =
            connector.connect(server_name, stream).await.unwrap();

        Ok(tls_stream)
    }
}