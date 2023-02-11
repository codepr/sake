mod connack;
mod connect;
use bytes::{BufMut, BytesMut};
pub use connack::ConnackPacket;
pub use connect::ConnectPacket;
use core::fmt::{self, Display, Formatter};
use std::slice::Iter;

/// Error during serialization and deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BoundaryCrossed(usize),
    InsufficientBytes(usize),
    MalformedPacket,
    MalformedRemainingLength,
    PayloadTooLong,
    StringNotUtf8,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error = {:?}", self)
    }
}

pub mod protocol {

    use crate::mqtt::Error;
    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use std::slice::Iter;

    const MAX_PAYLOAD_SIZE: usize = 268_435_455;

    /// Parses variable byte integer in the stream and returns the length
    /// and number of bytes that make it. Used for remaining length calculation
    /// as well as for calculating property lengths
    pub fn read_remaining_length(stream: &mut Iter<u8>) -> Result<(usize, usize), Error> {
        let mut len: usize = 0;
        let mut pos = 0;
        let mut done = false;
        let mut shift = 0;

        // Use continuation bit at position 7 to continue reading next
        // byte to frame 'length'.
        // Stream 0b1xxx_xxxx 0b1yyy_yyyy 0b1zzz_zzzz 0b0www_wwww will
        // be framed as number 0bwww_wwww_zzz_zzzz_yyy_yyyy_xxx_xxxx
        for byte in stream {
            pos += 1;
            let byte = *byte as usize;
            len += (byte & 0x7F) << shift;

            // stop when continue bit is 0
            done = (byte & 0x80) == 0;
            if done {
                break;
            }

            shift += 7;

            // Only a max of 4 bytes allowed for remaining length
            // more than 4 shifts (0, 7, 14, 21) implies bad length
            if shift > 21 {
                return Err(Error::MalformedRemainingLength);
            }
        }

        // Not enough bytes to frame remaining length. wait for
        // one more byte
        if !done {
            return Err(Error::InsufficientBytes(1));
        }

        Ok((pos, len))
    }

    /// Writes remaining length to stream and returns number of bytes for remaining length
    /// following the OASIS specs, encode the total length of the packet excluding header
    /// and the space for the encoding itself into a 1-4 bytes using the continuation
    /// bit technique to allow a dynamic storing size:
    ///
    /// Using the first 7 bits of a byte we can store values till 127 and use the
    /// last bit as a switch to notify if the subsequent byte is used to store
    /// remaining length or not.
    /// Returns the number of bytes used to store the value passed.
    pub fn write_remaining_length(stream: &mut BytesMut, len: usize) -> Result<usize, Error> {
        if len > MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadTooLong);
        }

        let mut done = false;
        let mut x = len;
        let mut count = 0;

        while !done {
            let mut byte = (x % 128) as u8;
            x /= 128;
            if x > 0 {
                byte |= 128;
            }

            stream.put_u8(byte);
            count += 1;
            done = x == 0;
        }

        Ok(count)
    }

    /// Reads a series of bytes with a length from a byte stream
    pub fn read_bytes(stream: &mut Bytes) -> Result<Bytes, Error> {
        if stream.len() < 2 {
            return Err(Error::MalformedPacket);
        }
        let len = stream.get_u16() as usize;

        // Prevent attacks with wrong remaining length. This method is used in
        // `packet.assembly()` with (enough) bytes to frame packet. Ensures that
        // reading variable len string or bytes doesn't cross promised boundary
        // with `read_fixed_header()`
        if len > stream.len() {
            return Err(Error::BoundaryCrossed(len));
        }

        Ok(stream.split_to(len))
    }

    /// Reads a string from bytes stream
    pub fn read_string(stream: &mut Bytes) -> Result<String, Error> {
        let s = read_bytes(stream)?;
        match String::from_utf8(s.to_vec()) {
            Ok(v) => Ok(v),
            Err(_e) => Err(Error::StringNotUtf8),
        }
    }

    /// Serializes bytes to stream (including length)
    pub fn write_bytes(stream: &mut BytesMut, bytes: &[u8]) {
        stream.put_u16(bytes.len() as u16);
        stream.extend_from_slice(bytes);
    }

    /// Serializes a string to stream
    pub fn write_string(stream: &mut BytesMut, string: &str) {
        write_bytes(stream, string.as_bytes());
    }
}

#[repr(u8)]
#[derive(PartialEq, PartialOrd, Debug)]
pub enum PacketType {
    Connect = 1,
    Connack,
    // Publish,
    // Puback,
    // Pubrec,
    // Pubrel,
    // Pubcomp,
    // Subscribe,
    // Suback,
    // Unsubscribe,
    // Unsuback,
    // PingReq,
    // PingResp,
    // Disconnect,
    Unknown,
}

impl PacketType {
    pub fn value(&self) -> u8 {
        match *self {
            PacketType::Connect => 0x01,
            PacketType::Connack => 0x02,
            PacketType::Unknown => 0xFF,
        }
    }
}

impl From<u8> for PacketType {
    fn from(orig: u8) -> Self {
        match orig {
            0x1 => return PacketType::Connect,
            0x2 => return PacketType::Connack,
            _ => return PacketType::Unknown,
        };
    }
}

#[repr(u8)]
pub enum Qos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, PartialEq)]
struct FixedHeaderFlags {
    retain: bool,
    qos: u8,
    dup: bool,
}

impl FixedHeaderFlags {
    pub fn new(retain: bool, qos: u8, dup: bool) -> Self {
        Self { retain, qos, dup }
    }

    pub fn from_byte(byte: u8) -> Self {
        let flag: Vec<bool> = (0..3).map(|i| byte & (u8::pow(2, i)) != 0).collect();
        let qos = flag[1..3].iter().fold(0, |acc, &b| acc * 2 + b as u8);
        println!("FROM_BYTE {} {} {}", flag[0], qos, flag[2]);
        Self::new(flag[0], qos, flag[2])
    }

    pub fn to_byte(&self) -> u8 {
        self.retain as u8 | (self.qos as u8) << 2 | (self.dup as u8) << 3
    }
}

/// MQTT Fixed header, according to official docs it's comprised of a single
/// byte carrying:
/// - opcode (packet type)
/// - dup flag
/// - QoS
/// - retain flag
/// It's followed by the remaining_len of the packet, encoded onto 1 to 4
/// bytes starting at bytes 2.
///
/// |   Bit      |  7  |  6  |  5  |  4  |  3  |  2  |  1  |   0    |
/// |------------|-----------------------|--------------------------|
/// | Byte 1     |      MQTT type 3      | dup |    QoS    | retain |
/// |------------|--------------------------------------------------|
/// | Byte 2     |                                                  |
/// |   .        |               Remaining Length                   |
/// |   .        |                                                  |
/// | Byte 5     |                                                  |
/// |------------|--------------------------------------------------|
#[derive(Debug, PartialEq)]
pub struct FixedHeader {
    packet_type: PacketType,
    flags: FixedHeaderFlags,
    remaining_length: u32,
}

impl fmt::Display for FixedHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.packet_type {
            PacketType::Connect => write!(
                f,
                "CONNECT: d:{} q:{} r:{}",
                self.flags.dup, self.flags.qos, self.flags.retain
            ),
            PacketType::Connack => write!(f, "CONNACK"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

impl FixedHeader {
    pub fn new(byte: u8, remaining_length: u32) -> FixedHeader {
        FixedHeader {
            packet_type: PacketType::from(byte >> 4),
            flags: FixedHeaderFlags::from_byte(byte),
            remaining_length,
        }
    }

    pub fn remaining_length(&self) -> u32 {
        self.remaining_length
    }

    pub fn from_stream(stream: &mut Iter<u8>) -> Result<FixedHeader, Error> {
        // At least 2 bytes are necessary to frame a packet
        let stream_len = stream.len();
        if stream_len < 2 {
            return Err(Error::InsufficientBytes(2 - stream_len));
        }

        if let Some(opcode) = stream.next() {
            let (_len_len, len) = protocol::read_remaining_length(stream)?;
            println!("{}: {}", *opcode, len);
            Ok(FixedHeader::new(*opcode, len as u32))
        } else {
            Err(Error::MalformedPacket)
        }
    }

    pub fn write(&self, buffer: &mut BytesMut) -> Result<usize, Error> {
        let len = self.remaining_length;
        // MSB for the MQTT type and LSB for the flags
        let byte = (self.packet_type.value()) << 4 | (self.flags.to_byte() & 0x0F);
        buffer.put_u8(byte);
        let count = protocol::write_remaining_length(buffer, len as usize)?;
        protocol::write_string(buffer, "MQTT");
        // protocol V4
        buffer.put_u8(0x04);
        Ok(count)
    }
}

#[cfg(test)]
mod fixed_headers_tests {
    use super::*;

    use bytes::{Bytes, BytesMut};

    #[test]
    fn test_new() {
        let fixed_header = FixedHeader::new(0x10, 18);
        assert_eq!(
            fixed_header,
            FixedHeader {
                packet_type: PacketType::Connect,
                flags: FixedHeaderFlags {
                    retain: false,
                    qos: 0,
                    dup: false
                },
                remaining_length: 18
            }
        );
    }

    #[test]
    fn test_from_stream() {
        let stream = Bytes::from_static(b"\x10\x12\0\x04MQTT\x04");
        let fixed_header = FixedHeader::from_stream(&mut stream.iter()).unwrap();
        assert_eq!(
            fixed_header,
            FixedHeader {
                packet_type: PacketType::Connect,
                flags: FixedHeaderFlags {
                    retain: false,
                    qos: 0,
                    dup: false
                },
                remaining_length: 18
            }
        );
    }

    #[test]
    fn test_write() {
        let fixed_header = FixedHeader::new(0x10, 18);
        let mut buffer = BytesMut::new();
        fixed_header.write(&mut buffer).unwrap();
        assert_eq!(buffer, Bytes::from_static(b"\x10\x12\0\x04MQTT\x04"));
    }
}
