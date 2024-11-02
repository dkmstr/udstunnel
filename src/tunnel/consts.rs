pub const HANDSHAKE_V1: &[u8] = b"\x5AMGB\xA5\x01\x00";
pub const TICKET_LENGTH: usize = 48;
pub const COMMAND_LENGTH: usize = 4;
pub const VERSION: &str = "v5.0.0";
pub const USER_AGENT: &str = "UDSTunnel/v5.0.0";

pub const COMMAND_OPEN: &str = "OPEN";
pub const COMMAND_TEST: &str = "TEST";
pub const COMMAND_STAT: &str = "STAT";
pub const COMMAND_INFO: &str = "INFO";

pub const RESPONSE_ERROR_TICKET: &str = "ERROR_TICKET";
pub const RESPONSE_ERROR_COMMAND: &str = "ERROR_COMMAND";
pub const RESPONSE_ERROR_TIMEOUT: &str = "TIMEOUT";
pub const RESPONSE_FORBIDDEN: &str = "FORBIDDEN";
pub const RESPONSE_OK: &str = "OK";
