use byteorder::ReadBytesExt;
use std::fmt;
use std::io::{self, Read};

/// Return code in connack
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ConnectReturnCode {
    Success = 0,
    RefusedProtocolVersion,
    BadClientId,
    ServiceUnavailable,
    BadUserNamePassword,
    NotAuthorized,
    Unknown,
}

impl fmt::Display for ConnectReturnCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectReturnCode::Success => write!(f, "Success"),
            ConnectReturnCode::RefusedProtocolVersion => write!(f, "Refused Protocol Version"),
            ConnectReturnCode::BadClientId => write!(f, "Bad Client ID"),
            ConnectReturnCode::ServiceUnavailable => write!(f, "Service Unavailable"),
            ConnectReturnCode::BadUserNamePassword => write!(f, "Bad Username or Password"),
            ConnectReturnCode::NotAuthorized => write!(f, "Not Authorized"),
            ConnectReturnCode::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ConnackPacket {
    pub session_present: bool,
    pub return_code: ConnectReturnCode,
}

impl fmt::Display for ConnackPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CONNACK: {} Session present: {}",
            self.return_code, self.session_present
        )
    }
}

impl ConnackPacket {
    pub fn from_bytes(bytes: &mut impl Read) -> io::Result<ConnackPacket> {
        let session_present = bytes.read_u8()? != 0;
        let return_code = match bytes.read_u8()? {
            0 => ConnectReturnCode::Success,
            1 => ConnectReturnCode::RefusedProtocolVersion,
            2 => ConnectReturnCode::BadClientId,
            3 => ConnectReturnCode::ServiceUnavailable,
            4 => ConnectReturnCode::BadUserNamePassword,
            5 => ConnectReturnCode::NotAuthorized,
            _ => ConnectReturnCode::Unknown,
        };
        Ok(ConnackPacket {
            session_present,
            return_code,
        })
    }
}

#[cfg(test)]
mod connack_tests {
    use super::*;
    use byteorder::WriteBytesExt;

    #[test]
    fn test_from_stream() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        buf.write_u8(0)?;
        buf.write_u8(0)?;

        let connack = ConnackPacket::from_bytes(&mut buf.as_slice()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                session_present: false,
                return_code: ConnectReturnCode::Success
            }
        );
        Ok(())
    }

    #[test]
    fn test_from_stream_session() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        buf.write_u8(1)?;
        buf.write_u8(0)?;

        let connack = ConnackPacket::from_bytes(&mut buf.as_slice()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                session_present: true,
                return_code: ConnectReturnCode::Success
            }
        );
        Ok(())
    }

    #[test]
    fn test_from_stream_return_code_refused_protocol_version() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        buf.write_u8(1)?;
        buf.write_u8(1)?;

        let connack = ConnackPacket::from_bytes(&mut buf.as_slice()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                session_present: true,
                return_code: ConnectReturnCode::RefusedProtocolVersion
            }
        );
        Ok(())
    }
}
