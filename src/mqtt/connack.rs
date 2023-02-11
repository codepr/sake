use crate::mqtt::{packet, protocol, Error};
use std::slice::Iter;

use std::fmt;

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
    pub fixed_header: packet::FixedHeader,
    pub session_present: bool,
    pub return_code: ConnectReturnCode,
}

impl fmt::Display for ConnackPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CONNACK: {} Session present: {} ({} bytes)",
            self.return_code,
            self.session_present,
            self.fixed_header.remaining_length()
        )
    }
}

impl ConnackPacket {
    pub fn from_stream(mut stream: Iter<u8>) -> Result<ConnackPacket, Error> {
        let fixed_header = packet::FixedHeader::from_stream(&mut stream)?;
        if let Some(session_present_u8) = stream.next() {
            let session_present = *session_present_u8 != 0;
            if let Some(rc) = stream.next() {
                let return_code = match *rc {
                    0 => ConnectReturnCode::Success,
                    1 => ConnectReturnCode::RefusedProtocolVersion,
                    2 => ConnectReturnCode::BadClientId,
                    3 => ConnectReturnCode::ServiceUnavailable,
                    4 => ConnectReturnCode::BadUserNamePassword,
                    5 => ConnectReturnCode::NotAuthorized,
                    _ => ConnectReturnCode::Unknown,
                };
                Ok(ConnackPacket {
                    fixed_header,
                    session_present,
                    return_code,
                })
            } else {
                Err(Error::MalformedPacket)
            }
        } else {
            Err(Error::MalformedPacket)
        }
    }
}

#[cfg(test)]
mod connack_tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn test_from_stream() {
        let mut stream = BytesMut::new();
        stream.put_u8(0x20);
        protocol::write_remaining_length(&mut stream, 2).unwrap();
        stream.put_u8(0);
        stream.put_u8(0);

        let connack = ConnackPacket::from_stream(stream.iter()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                fixed_header: packet::FixedHeader::new(0x20, 2),
                session_present: false,
                return_code: ConnectReturnCode::Success
            }
        );
    }

    #[test]
    fn test_from_stream_session() {
        let mut stream = BytesMut::new();
        stream.put_u8(0x20);
        protocol::write_remaining_length(&mut stream, 2).unwrap();
        stream.put_u8(1);
        stream.put_u8(0);

        let connack = ConnackPacket::from_stream(stream.iter()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                fixed_header: packet::FixedHeader::new(0x20, 2),
                session_present: true,
                return_code: ConnectReturnCode::Success
            }
        );
    }

    #[test]
    fn test_from_stream_return_code_refused_protocol_version() {
        let mut stream = BytesMut::new();
        stream.put_u8(0x20);
        protocol::write_remaining_length(&mut stream, 2).unwrap();
        stream.put_u8(1);
        stream.put_u8(1);

        let connack = ConnackPacket::from_stream(stream.iter()).unwrap();
        assert_eq!(
            connack,
            ConnackPacket {
                fixed_header: packet::FixedHeader::new(0x20, 2),
                session_present: true,
                return_code: ConnectReturnCode::RefusedProtocolVersion
            }
        );
    }
}
