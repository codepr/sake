use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug, PartialEq)]
pub struct PubcompPacket {
    pub packet_id: u16,
}

impl fmt::Display for PubcompPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PUBACK: packet ID {}", self.packet_id)
    }
}

impl PubcompPacket {
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
        let pubcomp = PubcompPacket { packet_id: 15 };
        let mut buf = vec![];
        pubcomp.write(&mut buf)?;
        assert_eq!(buf, &[0, 15]);
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> io::Result<()> {
        let bytes = &[2, 6];
        let pubcomp = PubcompPacket::from_bytes(&mut bytes.as_slice())?;
        assert_eq!(pubcomp, PubcompPacket { packet_id: 518 });
        Ok(())
    }
}
