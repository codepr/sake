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
use crate::mqtt::protocol;
use byteorder::{NetworkEndian, WriteBytesExt};
use std::fmt;
use std::io::{self, Write};

const MQTT_V4: u8 = 0x04;

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

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
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
        buf.write_u8(connect_flags)?;
        Ok(())
    }
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

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        self.flags.write(buf)?;
        buf.write_u16::<NetworkEndian>(self.keepalive)?;
        Ok(())
    }
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

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        if let Some(client_id) = &self.client_id {
            protocol::write_string(buf, client_id)?;
        }

        if let Some(will_topic) = &self.will_topic {
            protocol::write_string(buf, will_topic)?;
        }
        if let Some(will_message) = &self.will_message {
            protocol::write_string(buf, will_message)?;
        }

        if let Some(username) = &self.will_message {
            protocol::write_string(buf, username)?;
        }

        if let Some(password) = &self.will_message {
            protocol::write_string(buf, password)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct ConnectPacket {
    pub variable_header: ConnectVariableHeader,
    pub payload: ConnectPayload,
}

impl ConnectPacket {
    pub fn new(client_id: String, clean_session: bool) -> Self {
        Self {
            variable_header: ConnectVariableHeader::new(clean_session, 60),
            payload: ConnectPayload::new(client_id),
        }
    }

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        protocol::write_string(buf, "MQTT")?;
        buf.write_u8(MQTT_V4)?;
        self.variable_header.write(buf)?;
        self.payload.write(buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod connect_tests {
    use super::*;

    #[test]
    fn test_new() {
        let connect = ConnectPacket::new("test-id".into(), false);
        assert_eq!(
            connect,
            ConnectPacket {
                variable_header: ConnectVariableHeader::new(false, 60),
                payload: ConnectPayload::new("test-id".into())
            }
        );
    }

    #[test]
    fn test_write() {
        let connect = ConnectPacket::new("test-id".into(), false);
        let mut buffer = vec![];
        connect.write(&mut buffer).unwrap();
        assert_eq!(
            buffer,
            &[0, 4, 77, 81, 84, 84, 4, 0, 0, 60, 0, 7, 116, 101, 115, 116, 45, 105, 100]
        );
    }
}
