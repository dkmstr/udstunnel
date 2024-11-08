use std::fmt;

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
