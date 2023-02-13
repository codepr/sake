use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug, PartialEq)]
pub struct PubrelPacket {
    pub packet_id: u16,
}

impl fmt::Display for PubrelPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PUBACK: packet ID {}", self.packet_id)
    }
}

impl PubrelPacket {
    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        buf.write_u16::<NetworkEndian>(self.packet_id)?;
        Ok(())
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
    fn test_write() -> io::Result<()> {
        let pubrel = PubrelPacket { packet_id: 15 };
        let mut buf = vec![];
        pubrel.write(&mut buf)?;
        assert_eq!(buf, &[0, 15]);
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> io::Result<()> {
        let bytes = &[2, 6];
        let pubrel = PubrelPacket::from_bytes(&mut bytes.as_slice())?;
        assert_eq!(pubrel, PubrelPacket { packet_id: 518 });
        Ok(())
    }
}
