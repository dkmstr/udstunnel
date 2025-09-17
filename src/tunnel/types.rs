use std::{str::FromStr};
use tokio::io::AsyncReadExt;

use super::consts;

#[derive(Debug, PartialEq)]
pub enum Command {
    Open(String),
    Test,
    Stats(String),
    Unknown,
}

impl FromStr for Command {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < consts::COMMAND_LENGTH {
            return Err("Command too short");
        }

        match &s[..consts::COMMAND_LENGTH] {
            consts::COMMAND_OPEN => {
                // Get remainder of string after "OPEN " as the target (it's an ticket)
                // OPEN<ticket>
                // Ensure also that it has TICKET_LENGTH characters
                let ticket = s
                    .get(consts::COMMAND_OPEN.len()..)
                    .ok_or("Invalid command")?;
                if ticket.len() == consts::TICKET_LENGTH {
                    // Should match "^[a-zA-Z0-9]{48}$", 48 characters long and only ascii alphanumeric
                    if ticket.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Ok(Command::Open(ticket.to_string()));
                    }
                    Err("Invalid ticket, not alphanumeric")
                } else {
                    Err("Invalid ticket length")
                }
            }
            consts::COMMAND_TEST => Ok(Command::Test),
            consts::COMMAND_STATS | consts::COMMAND_INFO => {
                // Get remainder of the string after command that is the secret
                let secret = s
                    .get(consts::COMMAND_STATS.len()..)
                    .ok_or("Invalid command")?;
                if secret.len() == consts::SECRET_LENGTH {
                    // Should match "^[a-zA-Z0-9]{32}$", 32 characters long and only ascii alphanumeric
                    if secret.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Ok(Command::Stats(secret.to_string()));
                    }
                    Err("Invalid secret, not alphanumeric")
                } else {
                    Err("Invalid secret length")
                }
            }
            _ => Ok(Command::Unknown),
        }
    }
}

impl Command {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let command = String::from_utf8(bytes.to_vec()).unwrap();
        Command::from_str(&command)
    }

    pub async fn read_from_stream(
        stream: &mut tokio::net::TcpStream,
    ) -> Result<Self, &'static str> {
        let mut buffer = [0; consts::COMMAND_LENGTH];
        stream.readable().await.unwrap();
        match stream.read(&mut buffer).await {
            Ok(_) => {
                let command = String::from_utf8(buffer.to_vec()).unwrap();
                Command::from_str(&command)
            }
            Err(_) => Err("Failed to read from stream"),
        }
    }
}

impl From<&str> for Command {
    fn from(s: &str) -> Self {
        Command::from_str(s).unwrap_or(Command::Unknown)
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Open(ticket) => write!(f, "OPEN {}", ticket),
            Command::Test => write!(f, "TEST"),
            Command::Stats(secret) => write!(f, "STAT {}", secret),
            Command::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

pub enum Response {
    TicketError,
    CommandError,
    TimeoutError,
    HandshakeError,
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
            Response::HandshakeError => consts::RESPONSE_ERROR_HANDSHAKE,
            Response::ForbiddenError => consts::RESPONSE_FORBIDDEN,
            Response::ConnectError => consts::RESPONSE_ERROR_CONNECT,
            Response::Ok => consts::RESPONSE_OK,
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        self.to_string().as_bytes()
    }
}

impl From<Response> for String {
    fn from(response: Response) -> Self {
        response.to_string().to_string()
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
            Ok(Command::Open(
                "123456789012345678901234567890123456789012345678".to_string()
            ))
        );
        // Only 47 characters
        assert_eq!(
            Command::from_str("OPEN12345678901234567890123456789012345678901234567"),
            Err("Invalid ticket length")
        );
        // 49 characters
        assert_eq!(
            Command::from_str("OPEN1234567890123456789012345678901234567890123456789"),
            Err("Invalid ticket length")
        );
        assert_eq!(Command::from_str("TEST"), Ok(Command::Test));
        // Stat with 64 characters as secret
        assert_eq!(
            Command::from_str(
                "STAT1234567890123456789012345678901234567890123456789012345678901234"
            ),
            Ok(Command::Stats(
                "1234567890123456789012345678901234567890123456789012345678901234".to_string()
            ))
        );
        // Stat with 63 characters as secret
        assert_eq!(
            Command::from_str(
                "STAT123456789012345678901234567890123456789012345678901234567890123"
            ),
            Err("Invalid secret length")
        );
        // Stat with 65 characters as secret
        assert_eq!(
            Command::from_str(
                "STAT12345678901234567890123456789012345678901234567890123456789012345"
            ),
            Err("Invalid secret length")
        );
        assert_eq!(Command::from_str("INVALID"), Ok(Command::Unknown));

        // Test into
        let command: Command = "OPEN123456789012345678901234567890123456789012345678".into();
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
