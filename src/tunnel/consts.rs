use std::time::Duration;

pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

pub const BUFFER_SIZE: usize = 1024 * 16;
pub const HANDSHAKE_V1: &[u8] = b"\x5AMGB\xA5\x01\x00";
pub const TICKET_LENGTH: usize = 48;
pub const SECRET_LENGTH: usize = 64;
pub const COMMAND_LENGTH: usize = 4;
pub const VERSION: &str = "v5.0.0";
pub const USER_AGENT: &str = "UDSTunnel/v5.0.0";

pub const COMMAND_OPEN: &str = "OPEN";
pub const COMMAND_TEST: &str = "TEST";
pub const COMMAND_STATS: &str = "STAT";
pub const COMMAND_INFO: &str = "INFO";

pub const RESPONSE_ERROR_TICKET: &str = "ERROR_TICKET";
pub const RESPONSE_ERROR_COMMAND: &str = "ERROR_COMMAND";
pub const RESPONSE_ERROR_TIMEOUT: &str = "TIMEOUT";
pub const RESPONSE_FORBIDDEN: &str = "FORBIDDEN";
pub const RESPONSE_ERROR_CONNECT: &str = "ERROR_CONNECT";
pub const RESPONSE_OK: &str = "OK";

pub const CONFIGFILE: &str = "/etc/udstunnel.conf";
