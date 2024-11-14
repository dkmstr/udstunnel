use tokio::io::AsyncReadExt;

use super::consts;

#[derive(Debug, PartialEq)]
pub enum Command {
    Open(String),
    Test,
    Stat,
    Info,
    Unknown,
}

impl Command {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() < consts::COMMAND_LENGTH {
            return None;
        }

        match &s[..consts::COMMAND_LENGTH] {
            consts::COMMAND_OPEN => {
                // Get remainder of string after "OPEN " as the target (it's an ticket)
                // OPEN<ticket>
                // Ensure also that it has TICKET_LENGTH characters
                let ticket = s.get(consts::COMMAND_OPEN.len()..)?;
                if ticket.len() == consts::TICKET_LENGTH {
                    // Should match "^[a-zA-Z0-9]{48}$", 48 characters long and only ascii alphanumeric
                    if ticket.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Some(Command::Open(ticket.to_string()));
                    }
                    return None;  // Invalid ticket, not alphanumeric
                } else {
                    None
                }
            }
            consts::COMMAND_TEST => Some(Command::Test),
            consts::COMMAND_STAT => Some(Command::Stat),
            consts::COMMAND_INFO => Some(Command::Info),
            _ => Some(Command::Unknown),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let command = String::from_utf8(bytes.to_vec()).unwrap();
        Command::from_str(&command)
    }

    pub async fn read_from_stream(stream: &mut tokio::net::TcpStream) -> Option<Self> {
        let mut buffer = [0; consts::COMMAND_LENGTH];
        stream.readable().await.unwrap();
        match stream.read(&mut buffer).await {
            Ok(_) => {
                let command = String::from_utf8(buffer.to_vec()).unwrap();
                Command::from_str(&command)
            }
            Err(_) => None,
        }
    }
}

impl Into<Command> for String {
    fn into(self) -> Command {
        Command::from_str(&self).unwrap()
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Open(ticket) => write!(f, "OPEN {}", ticket),
            Command::Test => write!(f, "TEST"),
            Command::Stat => write!(f, "STAT"),
            Command::Info => write!(f, "INFO"),
            Command::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

pub enum Response {
    TicketError,
    CommandError,
    TimeoutError,
    ForbiddenError,
    ConnectError,
    Ok,
}

impl Response {
    pub fn to_string(&self) -> &str {
        match self {
            Response::TicketError => consts::RESPONSE_ERROR_TICKET,
            Response::CommandError => consts::RESPONSE_ERROR_COMMAND,
            Response::TimeoutError => consts::RESPONSE_ERROR_TIMEOUT,
            Response::ForbiddenError => consts::RESPONSE_FORBIDDEN,
            Response::ConnectError => consts::RESPONSE_ERROR_CONNECT,
            Response::Ok => consts::RESPONSE_OK,
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        self.to_string().as_bytes()
    }
}

impl Into<String> for Response {
    fn into(self) -> String {
        self.to_string().to_string()
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_from_str() {
        assert_eq!(
            Command::from_str("OPEN123456789012345678901234567890123456789012345678"),
            Some(Command::Open(
                "123456789012345678901234567890123456789012345678".to_string()
            ))
        );
        assert_eq!(
            Command::from_str("OPEN12345678901234567890123456789012345678901234567"),
            None
        );
        assert_eq!(
            Command::from_str("OPEN1234567890123456789012345678901234567890123456789"),
            None
        );
        assert_eq!(Command::from_str("TEST"), Some(Command::Test));
        assert_eq!(Command::from_str("STAT"), Some(Command::Stat));
        assert_eq!(Command::from_str("INFO"), Some(Command::Info));
        assert_eq!(Command::from_str("INVALID"), Some(Command::Unknown));

        // Test into
        let command: Command = "OPEN123456789012345678901234567890123456789012345678"
            .to_string()
            .into();
        assert_eq!(
            command,
            Command::Open("123456789012345678901234567890123456789012345678".to_string())
        );
    }

    #[test]
    fn test_response_to_string() {
        assert_eq!(
            Response::TicketError.to_string(),
            consts::RESPONSE_ERROR_TICKET
        );
        assert_eq!(
            Response::CommandError.to_string(),
            consts::RESPONSE_ERROR_COMMAND
        );
        assert_eq!(
            Response::TimeoutError.to_string(),
            consts::RESPONSE_ERROR_TIMEOUT
        );
        assert_eq!(
            Response::ForbiddenError.to_string(),
            consts::RESPONSE_FORBIDDEN
        );
        assert_eq!(Response::Ok.to_string(), consts::RESPONSE_OK);

        // Test into
        let response: String = Response::TicketError.into();
        assert_eq!(response, consts::RESPONSE_ERROR_TICKET);
    }
}
