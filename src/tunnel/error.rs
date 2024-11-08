use std::fmt;

use log;
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[derive(Debug)]
pub struct UDSError {
    message: String,
}

impl UDSError {
    pub fn new(message: &str) -> Self {
        UDSError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for UDSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UDSError: {}", self.message)
    }
}

impl std::error::Error for UDSError {}

pub(crate) async fn log_handshake_error(stream: &TcpStream, bytes: &[u8], timeout: bool) -> () {
    // Generate hex representation of the first 16 bytes
    let from = stream
        .peer_addr()
        .unwrap_or_else(|_| "[unknown]".parse().unwrap());

    let msg;
    if timeout {
        msg = format!("invalid from {from}: timed out");
    } else {
        let hex = bytes[0..std::cmp::min(bytes.len(), 16)]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("");
        msg = format!("invalid from {from}: invalid data: {hex}");
    }
    log::error!("HANDSHAKE: {}", msg);

}
