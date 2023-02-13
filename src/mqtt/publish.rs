use crate::mqtt::{protocol, FixedHeader};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug, PartialEq)]
pub struct PublishPacket {
    pub packet_id: u16,
    pub qos: u8,
    pub topic: String,
    pub payload: Vec<u8>,
}

impl fmt::Display for PublishPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PUBLISH: Packet ID: {} Topic: {}",
            self.packet_id, self.topic
        )
    }
}

impl PublishPacket {
    pub fn new(packet_id: u16, topic: String, payload: Vec<u8>, qos: u8) -> Self {
        Self {
            packet_id,
            qos,
            topic,
            payload,
        }
    }

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        protocol::write_string(buf, &self.topic)?;
        if self.qos > 0 {
            buf.write_u16::<NetworkEndian>(self.packet_id)?;
        }
        protocol::write_bytes(buf, &self.payload)?;
        Ok(())
    }

    pub fn from_bytes(buf: &mut impl Read, fixed_header: &FixedHeader) -> io::Result<Self> {
        let topic = protocol::read_string(buf)?;
        let mut bytes_read = 2 + topic.len();
        let packet_id = if fixed_header.flags.qos > 0 {
            bytes_read += 2;
            buf.read_u16::<NetworkEndian>()?
        } else {
            0
        };
        let mut payload_bytes =
            vec![0u8; (fixed_header.remaining_length() - (bytes_read as u32)) as usize];
        buf.read_exact(&mut payload_bytes)?;
        Ok(Self {
            packet_id,
            qos: fixed_header.flags.qos,
            topic,
            payload: payload_bytes,
        })
    }
}
