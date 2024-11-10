use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::utils;

pub struct Remote {
    pub port: u16,
    pub data: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[allow(dead_code)]
impl Remote {
    pub fn new(port: Option<u16>) -> Self {
        let port = port.unwrap_or(utils::find_free_port(None));
        Remote {
            port,
            data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn spawn(&self) -> tokio::task::JoinHandle<()> {
        let data = self.data.clone();
        let port = self.port;
        tokio::spawn(async move {
            Remote::listen(data, port).await;
        })
    }

    async fn listen(data: Arc<Mutex<Vec<Vec<u8>>>>, port: u16) {
        let listener = tokio::net::TcpListener::bind(format!("[::1]:{}", port))
            .await
            .unwrap();

        loop {
            // Wait for a connection
            let (mut stream, _) = listener.accept().await.unwrap();

            loop {
                let mut buffer = [0; 8192];
                let n = stream.read(&mut buffer).await.unwrap();
                if n == 0 {
                    stream.shutdown().await.unwrap();
                    break;
                }
                // Echo received data
                stream.write_all(&buffer[..n]).await.unwrap();

                data.lock().unwrap().push(buffer[..n].to_vec());
            }
        }
    }
}
