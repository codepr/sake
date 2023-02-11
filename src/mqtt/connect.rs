///
/// MQTT Connect packet, contains a variable header with some connect related
/// flags:
/// - clean session flag
/// - will flag
/// - will QoS (if will flag set to true)
/// - will topic (if will flag set to true)
/// - will retain flag (if will flag set to true)
/// - password flag
/// - username flag
///
/// It's followed by all required fields according the flags set to true.
///
/// |------------|--------------------------------------------------|
/// | Byte 6     |             Protocol name len MSB                |
/// | Byte 7     |             Protocol name len LSB                |  [UINT16]
/// |------------|--------------------------------------------------|
/// | Byte 8     |                                                  |
/// |   .        |                'M' 'Q' 'T' 'T'                   |
/// | Byte 12    |                                                  |
/// |------------|--------------------------------------------------|
/// | Byte 13    |                 Protocol level                   |
/// |------------|--------------------------------------------------|
/// |            |                 Connect flags                    |
/// | Byte 14    |--------------------------------------------------|
/// |            |  U  |  P  |  WR |     WQ    |  WF |  CS |    R   |
/// |------------|--------------------------------------------------|
/// | Byte 15    |                 Keepalive MSB                    |  [UINT16]
/// | Byte 17    |                 Keepalive LSB                    |
/// |------------|--------------------------------------------------|<-- Payload
/// | Byte 18    |             Client ID length MSB                 |  [UINT16]
/// | Byte 19    |             Client ID length LSB                 |
/// |------------|--------------------------------------------------|
/// | Byte 20    |                                                  |
/// |   .        |                  Client ID                       |
/// | Byte N     |                                                  |
/// |------------|--------------------------------------------------|
/// | Byte N+1   |              Username length MSB                 |
/// | Byte N+2   |              Username length LSB                 |
/// |------------|--------------------------------------------------|
/// | Byte N+3   |                                                  |
/// |   .        |                  Username                        |
/// | Byte N+M   |                                                  |
/// |------------|--------------------------------------------------|
/// | Byte N+M+1 |              Password length MSB                 |
/// | Byte N+M+2 |              Password length LSB                 |
/// |------------|--------------------------------------------------|
/// | Byte N+M+3 |                                                  |
/// |   .        |                  Password                        |
/// | Byte N+M+K |                                                  |
/// |------------|--------------------------------------------------|
///
use crate::mqtt::{protocol, Error, FixedHeader};
use bytes::{BufMut, BytesMut};
use std::fmt;

#[derive(Debug, PartialEq)]
struct ConnectFlags {
    clean_session: bool,
    will: bool,
    will_qos: u8,
    will_retain: bool,
    password: bool,
    username: bool,
}

impl fmt::Display for ConnectFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "clean session:{} will:{} will_qos:{} will_retain: {} username: {} password: {}",
            self.clean_session,
            self.will,
            self.will_qos,
            self.will_retain,
            self.username,
            self.password
        )
    }
}

impl ConnectFlags {
    pub fn new(clean_session: bool) -> ConnectFlags {
        ConnectFlags {
            clean_session,
            will: false,
            will_qos: 0,
            will_retain: false,
            password: false,
            username: false,
        }
    }

    pub fn will(&self) -> bool {
        self.will
    }

    pub fn username(&self) -> bool {
        self.username
    }

    pub fn password(&self) -> bool {
        self.password
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<usize, Error> {
        let mut connect_flags = 0;
        if self.clean_session {
            connect_flags |= 0x02;
        }
        if self.will {
            connect_flags |= 0x04;
        }
        if self.username {
            connect_flags |= 0x80;
        }
        if self.password {
            connect_flags |= 0x40;
        }
        buffer.put_u8(connect_flags);
        Ok(1)
    }

    // pub fn from_u8(byte: u8) -> ConnectFlags {
    //     let bits: Vec<bool> = (0..8).map(|i| byte & (u8::pow(2, i)) != 0).collect();
    //     ConnectFlags {
    //         clean_session: bits[1],
    //         will: bits[2],
    //         will_qos: bits[3..5].iter().fold(0, |acc, &b| acc * 2 + b as u8),
    //         will_retain: bits[5],
    //         password: bits[6],
    //         username: bits[7],
    //     }
    // }
}

#[derive(Debug, PartialEq)]
pub struct ConnectVariableHeader {
    flags: ConnectFlags,
    keepalive: u16,
}

impl fmt::Display for ConnectVariableHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} keepalive:{}", self.flags, self.keepalive,)
    }
}

impl ConnectVariableHeader {
    pub fn new(clean_session: bool, keepalive: u16) -> ConnectVariableHeader {
        ConnectVariableHeader {
            flags: ConnectFlags::new(clean_session),
            keepalive,
        }
    }
    pub fn will(&self) -> bool {
        self.flags.will()
    }

    pub fn username(&self) -> bool {
        self.flags.username()
    }

    pub fn password(&self) -> bool {
        self.flags.password()
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<usize, Error> {
        let flags_size = self.flags.write(buffer)?;
        buffer.put_u16(self.keepalive);
        Ok(flags_size + 2)
    }

    // pub fn from_binary(buf: &mut BytesMut) -> SerdeResult<ConnectVariableHeader> {
    //     Ok(ConnectVariableHeader {
    //         flags: ConnectFlags::from_u8(buf.read_u8()?),
    //         keepalive: buf.read_u16()?,
    //     })
    // }
}

#[derive(Debug, PartialEq)]
pub struct ConnectPayload {
    client_id: Option<String>,
    will_topic: Option<String>,
    will_message: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl fmt::Display for ConnectPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cid = self.client_id.as_deref().unwrap_or("");
        let topic = self.will_topic.as_deref().unwrap_or("");
        let message = self.will_message.as_deref().unwrap_or("");
        let user = self.username.as_deref().unwrap_or("");
        let pass = self.password.as_deref().unwrap_or("");
        write!(f, "{} {} {} {} {}", cid, topic, message, user, pass)
    }
}

impl ConnectPayload {
    pub fn new(client_id: String) -> ConnectPayload {
        ConnectPayload {
            client_id: Some(client_id),
            will_topic: None,
            will_message: None,
            username: None,
            password: None,
        }
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<usize, Error> {
        let mut len = 0;
        if let Some(client_id) = &self.client_id {
            protocol::write_string(buffer, client_id);
            len += client_id.len();
        }

        if let Some(will_topic) = &self.will_topic {
            protocol::write_string(buffer, will_topic);
            len += will_topic.len();
        }

        if let Some(will_message) = &self.will_message {
            protocol::write_string(buffer, will_message);
            len += will_message.len();
        }

        if let Some(username) = &self.will_message {
            protocol::write_string(buffer, username);
            len += username.len();
        }

        if let Some(password) = &self.will_message {
            protocol::write_string(buffer, password);
            len += password.len();
        }

        Ok(len)
    }
}

#[derive(Debug, PartialEq)]
pub struct ConnectPacket {
    pub fixed_header: FixedHeader,
    pub variable_header: ConnectVariableHeader,
    pub payload: ConnectPayload,
}

impl ConnectPacket {
    pub fn new(client_id: String, clean_session: bool) -> Self {
        let len = 10 + 2 + client_id.len();
        Self {
            fixed_header: FixedHeader::new(0x10, len as u32),
            variable_header: ConnectVariableHeader::new(clean_session, 60),
            payload: ConnectPayload::new(client_id),
        }
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<usize, Error> {
        self.fixed_header.write(buffer)?;
        self.variable_header.write(buffer)?;
        self.payload.write(buffer)?;
        Ok(1)
    }
}

#[cfg(test)]
mod connect_tests {
    use super::*;

    use bytes::{Bytes, BytesMut};

    #[test]
    fn test_new() {
        let connect = ConnectPacket::new("test-id".into(), false);
        assert_eq!(
            connect,
            ConnectPacket {
                fixed_header: FixedHeader::new(0x10, 19),
                variable_header: ConnectVariableHeader::new(false, 60),
                payload: ConnectPayload::new("test-id".into())
            }
        );
    }

    #[test]
    fn test_write() {
        let connect = ConnectPacket::new("test-id".into(), false);
        let mut buffer = BytesMut::new();
        connect.write(&mut buffer).unwrap();
        assert_eq!(
            buffer,
            Bytes::from_static(b"\x10\x13\0\x04MQTT\x04\0\0<\0\x07test-id")
        );
    }
}
