use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug, PartialEq)]
pub struct PubrecPacket {
    pub packet_id: u16,
}

impl fmt::Display for PubrecPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PUBACK: packet ID {}", self.packet_id)
    }
}

impl PubrecPacket {
    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        buf.write_u16::<NetworkEndian>(self.packet_id)
    }

    pub fn from_bytes(bytes: &mut impl Read) -> io::Result<Self> {
        let packet_id = bytes.read_u16::<NetworkEndian>()?;
        Ok(Self { packet_id })
    }
}

#[cfg(test)]
mod puback_tests {
    use super::*;

    #[test]
    fn test_from_bytes() -> io::Result<()> {
        let bytes = &[2, 6];
        let pubrec = PubrecPacket::from_bytes(&mut bytes.as_slice())?;
        assert_eq!(pubrec, PubrecPacket { packet_id: 518 });
        Ok(())
    }
}
