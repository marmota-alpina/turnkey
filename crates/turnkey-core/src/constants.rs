/// Protocol delimiters
pub const DELIMITER_DEVICE: &str = "+";
pub const DELIMITER_FIELD: &str = "]";
pub const DELIMITER_SUBFIELD: &str = "[";
pub const DELIMITER_ARRAY: &str = "}";
pub const DELIMITER_NESTED: &str = "{";

/// Protocol markers
pub const PROTOCOL_ID: &str = "REON";

/// Message structure
pub const START_BYTE: u8 = 0x02; // STX
pub const END_BYTE: u8 = 0x03; // ETX

/// Frame overhead (STX + ETX = 2 bytes)
pub const FRAME_OVERHEAD: usize = 2;

/// Frame capacity calculation components
pub const DEVICE_ID_LENGTH: usize = 2; // Zero-padded to 2 digits (01-99)
pub const PROTOCOL_ID_LENGTH: usize = 4; // "REON"
pub const BASE_DELIMITER_COUNT: usize = 2; // Two '+' separators (ID+REON+CMD)

/// Timeouts (milliseconds)
pub const DEFAULT_ONLINE_TIMEOUT: u64 = 3000;
pub const MIN_ONLINE_TIMEOUT: u64 = 500;
pub const MAX_ONLINE_TIMEOUT: u64 = 10000;

/// Card format
pub const MIN_CARD_LENGTH: usize = 3;
pub const MAX_CARD_LENGTH: usize = 20;
pub const CARD_PADDED_LENGTH: usize = 20;

/// Device limits
pub const MIN_DEVICE_ID: u8 = 1;
pub const MAX_DEVICE_ID: u8 = 99;

/// Display message
pub const MAX_DISPLAY_MESSAGE_LENGTH: usize = 40;

/// Default messages
pub const MSG_ACCESS_GRANTED: &str = "Acesso liberado";
pub const MSG_ACCESS_DENIED: &str = "Acesso negado";
pub const MSG_WAITING: &str = "Aguardando...";
pub const MSG_TIMEOUT: &str = "Tempo esgotado";
