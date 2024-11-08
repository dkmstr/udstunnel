use std::fmt;

use log::error;
use tokio::net::TcpStream;

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

pub(crate) fn log_handshake_error(from: &TcpStream, bytes: &[u8]) -> () {
    // Generate hex representation of the first 16 bytes
    let hex = bytes[0..std::cmp::min(bytes.len(), 16)]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join(":");
    let from = from.peer_addr().unwrap_or_else(|_| "[unknown]".parse().unwrap());

    error!("HANDSHAKE error from {from}: {hex}");
}
