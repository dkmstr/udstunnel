use super::consts;

#[derive(Debug, PartialEq)]
pub enum Command {
    Open(String),
    Test,
    Stat,
    Info,
}

impl Command {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() < consts::COMMAND_LENGTH {
            return None;
        }

        match &s[..consts::COMMAND_LENGTH] {
            consts::COMMAND_OPEN => {
                // Get remainder of string after "OPEN " as the target (it's an ticket)
                // OPEN <ticket>
                // Ensure also that it has TICKET_LENGTH characters
                let ticket = s.get(consts::COMMAND_OPEN.len()+1..)?;
                if ticket.len() == consts::TICKET_LENGTH {
                    Some(Command::Open(ticket.to_string()))
                } else {
                    None
                }
            }
            consts::COMMAND_TEST => Some(Command::Test),
            consts::COMMAND_STAT => Some(Command::Stat),
            consts::COMMAND_INFO => Some(Command::Info),
            _ => None,
        }
    }
}

impl Into <Command> for String {
    fn into(self) -> Command {
        Command::from_str(&self).unwrap()
    }
}

pub enum Response {
    TicketError,
    CommandError,
    TimeoutError,
    ForbiddenError,
    Ok,
}

impl Response {
    pub fn to_string(&self) -> &str {
        match self {
            Response::TicketError => consts::RESPONSE_ERROR_TICKET,
            Response::CommandError => consts::RESPONSE_ERROR_COMMAND,
            Response::TimeoutError => consts::RESPONSE_ERROR_TIMEOUT,
            Response::ForbiddenError => consts::RESPONSE_FORBIDDEN,
            Response::Ok => consts::RESPONSE_OK,
        }
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
            Command::from_str("OPEN 123456789012345678901234567890123456789012345678"),
            Some(Command::Open(
                "123456789012345678901234567890123456789012345678".to_string()
            ))
        );
        assert_eq!(
            Command::from_str("OPEN 12345678901234567890123456789012345678901234567"),
            None
        );
        assert_eq!(
            Command::from_str("OPEN 1234567890123456789012345678901234567890123456789"),
            None
        );
        assert_eq!(Command::from_str("TEST"), Some(Command::Test));
        assert_eq!(Command::from_str("STAT"), Some(Command::Stat));
        assert_eq!(Command::from_str("INFO"), Some(Command::Info));
        assert_eq!(Command::from_str("INVALID"), None);

        // Test into
        let command: Command = "OPEN 123456789012345678901234567890123456789012345678".to_string().into();
        assert_eq!(
            command,
            Command::Open(
                "123456789012345678901234567890123456789012345678".to_string()
            )
        );
    }

    #[test]
    fn test_response_to_string() {
        assert_eq!(Response::TicketError.to_string(), consts::RESPONSE_ERROR_TICKET);
        assert_eq!(Response::CommandError.to_string(), consts::RESPONSE_ERROR_COMMAND);
        assert_eq!(Response::TimeoutError.to_string(), consts::RESPONSE_ERROR_TIMEOUT);
        assert_eq!(Response::ForbiddenError.to_string(), consts::RESPONSE_FORBIDDEN);
        assert_eq!(Response::Ok.to_string(), consts::RESPONSE_OK);

        // Test into
        let response: String = Response::TicketError.into();
        assert_eq!(response, consts::RESPONSE_ERROR_TICKET);
    }
}
