use serde::{Deserialize, Serialize};
use turnkey_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandCode {
    // Access control
    AccessRequest,    // 000+0
    GrantBoth,        // 00+1
    GrantManual,      // 00+4
    GrantEntry,       // 00+5
    GrantExit,        // 00+6
    DenyAccess,       // 00+30

    // Turnstile status
    WaitingRotation,  // 000+80
    RotationCompleted, // 000+81
    RotationTimeout,  // 000+82

    // Management
    SendConfig,       // EC
    SendCards,        // ECAR
    SendUsers,        // EU
    SendBiometrics,   // ED
    SendDateTime,     // EH
    ReceiveLogs,      // ER
    QueryStatus,      // RQ
    ReceiveConfig,    // RC
}

impl CommandCode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "000+0" => Ok(CommandCode::AccessRequest),
            "00+1" => Ok(CommandCode::GrantBoth),
            "00+4" => Ok(CommandCode::GrantManual),
            "00+5" => Ok(CommandCode::GrantEntry),
            "00+6" => Ok(CommandCode::GrantExit),
            "00+30" => Ok(CommandCode::DenyAccess),
            "000+80" => Ok(CommandCode::WaitingRotation),
            "000+81" => Ok(CommandCode::RotationCompleted),
            "000+82" => Ok(CommandCode::RotationTimeout),
            "EC" => Ok(CommandCode::SendConfig),
            "ECAR" => Ok(CommandCode::SendCards),
            "EU" => Ok(CommandCode::SendUsers),
            "ED" => Ok(CommandCode::SendBiometrics),
            "EH" => Ok(CommandCode::SendDateTime),
            "ER" => Ok(CommandCode::ReceiveLogs),
            "RQ" => Ok(CommandCode::QueryStatus),
            "RC" => Ok(CommandCode::ReceiveConfig),
            _ => Err(Error::InvalidCommandCode(s.to_string())),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CommandCode::AccessRequest => "000+0",
            CommandCode::GrantBoth => "00+1",
            CommandCode::GrantManual => "00+4",
            CommandCode::GrantEntry => "00+5",
            CommandCode::GrantExit => "00+6",
            CommandCode::DenyAccess => "00+30",
            CommandCode::WaitingRotation => "000+80",
            CommandCode::RotationCompleted => "000+81",
            CommandCode::RotationTimeout => "000+82",
            CommandCode::SendConfig => "EC",
            CommandCode::SendCards => "ECAR",
            CommandCode::SendUsers => "EU",
            CommandCode::SendBiometrics => "ED",
            CommandCode::SendDateTime => "EH",
            CommandCode::ReceiveLogs => "ER",
            CommandCode::QueryStatus => "RQ",
            CommandCode::ReceiveConfig => "RC",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_code_parse() {
        assert_eq!(CommandCode::parse("000+0").unwrap(), CommandCode::AccessRequest);
        assert_eq!(CommandCode::parse("00+6").unwrap(), CommandCode::GrantExit);
        assert_eq!(CommandCode::parse("000+81").unwrap(), CommandCode::RotationCompleted);
        assert_eq!(CommandCode::parse("ECAR").unwrap(), CommandCode::SendCards);
    }

    #[test]
    fn test_command_code_invalid() {
        assert!(CommandCode::parse("INVALID").is_err());
    }

    #[test]
    fn test_command_code_round_trip() {
        let commands = vec![
            CommandCode::AccessRequest,
            CommandCode::GrantExit,
            CommandCode::WaitingRotation,
            CommandCode::SendCards,
        ];

        for cmd in commands {
            let str_repr = cmd.as_str();
            let parsed = CommandCode::parse(str_repr).unwrap();
            assert_eq!(parsed, cmd);
        }
    }
}
