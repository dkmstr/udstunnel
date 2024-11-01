use tokio::{io, net::TcpStream};
use async_trait::async_trait;

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

use super::noverify::NoVerifySsl;

#[async_trait]
pub trait TLSPreOperation: Send + Sync {
    async fn pre_tls(&self, stream: &mut TcpStream) -> io::Result<()>;
}

pub struct ConnectionBuilder{
    server: String,
    port: u16,
    verify: bool,
    precondition: Option<Box<dyn TLSPreOperation>>,
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
            precondition: None,
        }
    }

    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_pretls<T: TLSPreOperation + 'static>(mut self, precondition: T) -> Self {
        self.precondition = Some(Box::new(precondition));
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
        let mut stream = TcpStream::connect(format!("{}:{}", self.server, self.port))
            .await
            .unwrap();

        if let Some(precondition) = self.precondition {
            precondition.pre_tls(&mut stream).await?;
        }

        let tls_stream: tokio_rustls::client::TlsStream<TcpStream> =
            connector.connect(server_name, stream).await.unwrap();

        Ok(tls_stream)
    }
}
