mod connack;
mod connect;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod subscribe;
use byteorder::{ReadBytesExt, WriteBytesExt};
use connack::ConnackPacket;
use connect::ConnectPacket;
use core::fmt::{self, Display, Formatter};
use puback::PubackPacket;
use pubcomp::PubcompPacket;
use publish::PublishPacket;
use pubrec::PubrecPacket;
use pubrel::PubrelPacket;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::net::TcpStream;
use subscribe::{SubscribePacket, SubscriptionTopic};

/// Error during serialization and deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    PayloadTooLong,
}

impl Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error = {:?}", self)
    }
}

impl Error for TransportError {}

pub mod protocol {

    use crate::mqtt::TransportError;
    use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
    use std::io::{self, Read, Write};

    const MAX_PAYLOAD_SIZE: usize = 268_435_455;
    pub const MQTT_HEADER_LEN: usize = 2;

    /// Parses variable byte integer in the stream and returns the length
    /// and number of bytes that make it. Used for remaining length calculation
    /// as well as for calculating property lengths
    pub fn read_remaining_length(buf: &mut impl Read) -> io::Result<u32> {
        let mut c = buf.read_u8()?;
        let mut mul = 1u64;
        let mut val = if c & 128 == 0 { (c & 127) as u32 } else { 0u32 };

        // stop when continue bit is 0
        while (c & 128) != 0 {
            val += ((c & 127) as u64 * mul) as u32;
            mul *= 128;
            c = buf.read_u8()?;
        }
        Ok(val)
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
    pub fn write_remaining_length(buf: &mut impl Write, len: usize) -> io::Result<usize> {
        if len > MAX_PAYLOAD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                TransportError::PayloadTooLong,
            ));
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

            buf.write_u8(byte)?;
            count += 1;
            done = x == 0;
        }

        Ok(count)
    }

    ///
    /// Returns the size of a packet. Useful to pack functions to know the expected
    /// buffer size of the packet based on the opcode. Accept an optional pointer
    /// to get the len reserved for storing the remaining length of the full packet
    /// excluding the fixed header (1 byte) and the bytes needed to store the
    /// value itself (1 to 3 bytes).
    ///
    pub fn mqtt_remaining_length_size(len: usize) -> usize {
        //
        // Here we take into account the number of bytes needed to store the total
        // amount of bytes size of the packet, excluding the encoding space to
        // store the value itself and the fixed header, updating len pointer if
        // not NULL.
        //
        if (len - 1) > 0x200000 {
            // 3 bytes <= 128 * 128 * 128
            3
        } else if (len - 1) > 0x4000 {
            // 2 bytes <= 128 * 128
            2
        } else if (len - 1) > 0x80 {
            // 1 byte  <= 128
            1
        } else {
            0
        }
    }

    /// Reads a series of bytes with a length from a byte stream
    pub fn read_string(buf: &mut impl Read) -> io::Result<String> {
        // byteorder ReadBytesExt
        let length = buf.read_u16::<NetworkEndian>()?;

        // Given the length of our string, only read in that quantity of bytes
        let mut bytes = vec![0u8; length as usize];
        buf.read_exact(&mut bytes)?;

        // And attempt to decode it as UTF8
        String::from_utf8(bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8"))
    }

    /// Serializes bytes to stream (including length)
    pub fn write_bytes(buf: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
        buf.write_all(&bytes)
    }

    /// Serializes a string to stream
    pub fn write_string(buf: &mut impl Write, string: &str) -> io::Result<()> {
        let message = string.as_bytes();
        buf.write_u16::<NetworkEndian>(message.len() as u16)?;
        buf.write_all(&message)
    }
}

#[repr(u8)]
#[derive(PartialEq, PartialOrd, Debug, Copy, Clone)]
pub enum PacketType {
    Connect = 1,
    Connack,
    Publish,
    Puback,
    Pubrec,
    Pubrel,
    Pubcomp,
    Subscribe,
    // Suback,
    // Unsubscribe,
    // Unsuback,
    // PingReq,
    // PingResp,
    Disconnect,
    Unknown,
}

#[repr(u8)]
pub enum AckType {
    Puback(u16),
    Pubrec(u16),
    Pubrel(u16),
    Pubcomp(u16),
}

impl From<&PacketType> for u8 {
    fn from(orig: &PacketType) -> Self {
        match orig {
            PacketType::Connect => 0x01,
            PacketType::Connack => 0x02,
            PacketType::Publish => 0x03,
            PacketType::Puback => 0x04,
            PacketType::Pubrec => 0x05,
            PacketType::Pubrel => 0x06,
            PacketType::Pubcomp => 0x07,
            PacketType::Subscribe => 0x08,
            PacketType::Disconnect => 0x0e,
            PacketType::Unknown => 0xFF,
        }
    }
}

impl From<u8> for PacketType {
    fn from(orig: u8) -> Self {
        match orig {
            0x1 => PacketType::Connect,
            0x2 => PacketType::Connack,
            0x3 => PacketType::Publish,
            0x4 => PacketType::Puback,
            0x5 => PacketType::Pubrec,
            0x6 => PacketType::Pubrel,
            0x7 => PacketType::Pubcomp,
            0x8 => PacketType::Subscribe,
            0xE => PacketType::Disconnect,
            _ => PacketType::Unknown,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum Qos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl From<u8> for Qos {
    fn from(orig: u8) -> Self {
        match orig {
            0 => Qos::AtMostOnce,
            1 => Qos::AtLeastOnce,
            2 => Qos::ExactlyOnce,
            n => panic!("Unknown QoS value: {}", n),
        }
    }
}

impl From<&Qos> for u8 {
    fn from(orig: &Qos) -> Self {
        match orig {
            Qos::AtMostOnce => 0,
            Qos::AtLeastOnce => 1,
            Qos::ExactlyOnce => 2,
        }
    }
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
        let flag: Vec<bool> = (0..4).map(|i| byte & (u8::pow(2, i)) != 0).collect();
        let qos = flag[1..3].iter().enumerate().fold(0, |acc, (i, &b)| {
            acc + (u8::pow(2, i as u32) * b as u8) as u8
        });
        Self::new(flag[0], qos, flag[3])
    }

    pub fn to_byte(&self) -> u8 {
        self.retain as u8 | (self.qos as u8) << 1 | (self.dup as u8) << 3
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
    pub packet_type: PacketType,
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

    pub fn from_bytes(bytes: &mut impl Read) -> io::Result<FixedHeader> {
        let opcode = bytes.read_u8()?;
        let len = protocol::read_remaining_length(bytes)?;
        Ok(FixedHeader::new(opcode, len as u32))
    }

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        let len = self.remaining_length;
        // MSB for the MQTT type and LSB for the flags
        let byte = (self.packet_type as u8) << 4 | (self.flags.to_byte() & 0x0F);
        buf.write_u8(byte)?;
        protocol::write_remaining_length(buf, len as usize)?;
        Ok(())
    }
}

/// Trait for something that can be converted to bytes (&[u8])
pub trait Serialize {
    /// Serialize to a `Write`able buffer
    fn serialize(&self, buf: &mut impl Write) -> io::Result<usize>;
}
/// Trait for something that can be converted from bytes (&[u8])
pub trait Deserialize {
    /// The type that this deserializes to
    type Output;

    /// Deserialize from a `Read`able buffer
    fn deserialize(buf: &mut impl Read) -> io::Result<Self::Output>;
}

#[derive(Debug)]
pub enum Request {
    Connect {
        client_id: String,
        clean_session: bool,
    },
    Publish {
        packet_id: u16,
        qos: u8,
        topic: String,
        payload: Vec<u8>,
    },
    Puback {
        packet_id: u16,
    },
    Pubrec {
        packet_id: u16,
    },
    Pubrel {
        packet_id: u16,
    },
    Pubcomp {
        packet_id: u16,
    },
    Subscribe {
        packet_id: u16,
        subscription_topics: Vec<SubscriptionTopic>,
    },
    Disconnect,
}

impl From<&Request> for u8 {
    fn from(req: &Request) -> Self {
        match req {
            Request::Connect { .. } => 0x10,
            Request::Publish { qos, .. } => encode_qos(0x30, Qos::from(*qos)),
            Request::Puback { .. } => 0x40,
            Request::Pubrec { .. } => 0x50,
            Request::Pubrel { .. } => 0x62,
            Request::Pubcomp { .. } => 0x70,
            Request::Subscribe { .. } => 0x80,
            Request::Disconnect => 0xE0,
        }
    }
}

fn encode_qos(byte: u8, qos: Qos) -> u8 {
    let mask1 = 1 << 1;
    let mask2 = 1 << 2;
    match qos {
        Qos::AtMostOnce => (byte & !mask1) & !mask2,
        Qos::AtLeastOnce => (byte & !mask2) | mask1,
        Qos::ExactlyOnce => (byte & !mask1) | mask2,
    }
}

impl Serialize for Request {
    fn serialize(&self, buf: &mut impl Write) -> io::Result<usize> {
        buf.write_u8(self.into())?;
        match self {
            Request::Connect {
                client_id,
                clean_session,
            } => {
                let len = 10 + 2 + client_id.len();
                protocol::write_remaining_length(buf, len)?;
                let connect = ConnectPacket::new(client_id.to_string(), *clean_session);
                connect.write(buf)?;
            }
            Request::Publish {
                packet_id,
                qos,
                topic,
                payload,
            } => {
                let len = 2 + topic.len() + payload.len() + if *qos > 0 { 2 } else { 0 };
                protocol::write_remaining_length(buf, len)?;
                let publish =
                    PublishPacket::new(*packet_id, topic.to_string(), payload.to_vec(), *qos);
                publish.write(buf)?;
            }
            Request::Puback { packet_id } => {
                let len = 2;
                protocol::write_remaining_length(buf, len)?;
                let puback = PubackPacket {
                    packet_id: *packet_id,
                };
                puback.write(buf)?;
            }
            Request::Pubrec { packet_id } => {
                let len = 2;
                protocol::write_remaining_length(buf, len)?;
                let pubrec = PubrecPacket {
                    packet_id: *packet_id,
                };
                pubrec.write(buf)?;
            }
            Request::Pubrel { packet_id } => {
                let len = 2;
                protocol::write_remaining_length(buf, len)?;
                let pubrel = PubrelPacket {
                    packet_id: *packet_id,
                };
                pubrel.write(buf)?;
            }
            Request::Pubcomp { packet_id } => {
                let len = 2;
                protocol::write_remaining_length(buf, len)?;
                let pubcomp = PubcompPacket {
                    packet_id: *packet_id,
                };
                pubcomp.write(buf)?;
            }
            Request::Subscribe {
                packet_id,
                subscription_topics,
            } => {
                let len = 2 + subscription_topics
                    .iter()
                    .map(|s| 2 + s.topic.len())
                    .sum::<usize>();
                protocol::write_remaining_length(buf, len)?;
                let subscribe = SubscribePacket {
                    packet_id: *packet_id,
                    subscription_topics: subscription_topics.to_vec(),
                };
                subscribe.write(buf)?;
            }
            Request::Disconnect => {
                let len = 0;
                protocol::write_remaining_length(buf, len)?;
            }
        }
        Ok(1)
    }
}

#[derive(Debug)]
pub enum Response {
    Connack {
        session_present: bool,
        return_code: u8,
    },
    Publish {
        packet_id: u16,
        qos: u8,
        topic: String,
        payload: Vec<u8>,
    },
    Puback {
        packet_id: u16,
    },
    Pubrec {
        packet_id: u16,
    },
    Pubrel {
        packet_id: u16,
    },
    Pubcomp {
        packet_id: u16,
    },
    Unknown,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Response::Connack {
                session_present,
                return_code,
            } => write!(f, "CONNACK {:?} {:?}", session_present, return_code),
            Response::Publish {
                packet_id,
                qos,
                topic,
                ..
            } => write!(f, "PUBLISH {:?} {} {}", packet_id, qos, topic),
            Response::Puback { packet_id } => write!(f, "PUBACK {:?}", packet_id),
            Response::Pubrec { packet_id } => write!(f, "PUBREC {:?}", packet_id),
            Response::Pubrel { packet_id } => write!(f, "PUBREL {:?}", packet_id),
            Response::Pubcomp { packet_id } => write!(f, "PUBCOMP {:?}", packet_id),
            Response::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

impl Deserialize for Response {
    type Output = Response;

    fn deserialize(buf: &mut impl Read) -> io::Result<Self::Output> {
        let fixed_header = FixedHeader::from_bytes(buf)?;
        let packet = match fixed_header.packet_type {
            PacketType::Connack => {
                let connack = ConnackPacket::from_bytes(buf)?;
                Response::Connack {
                    session_present: connack.session_present,
                    return_code: connack.return_code as u8,
                }
            }
            PacketType::Publish => {
                let publish = PublishPacket::from_bytes(buf, &fixed_header)?;
                Response::Publish {
                    packet_id: publish.packet_id,
                    qos: publish.qos,
                    topic: publish.topic,
                    payload: publish.payload,
                }
            }
            PacketType::Puback => {
                let puback = PubackPacket::from_bytes(buf)?;
                Response::Puback {
                    packet_id: puback.packet_id,
                }
            }
            PacketType::Pubrec => {
                let pubrec = PubrecPacket::from_bytes(buf)?;
                Response::Pubrec {
                    packet_id: pubrec.packet_id,
                }
            }
            PacketType::Pubrel => {
                let pubrel = PubrelPacket::from_bytes(buf)?;
                Response::Pubrel {
                    packet_id: pubrel.packet_id,
                }
            }
            PacketType::Pubcomp => {
                let pubcomp = PubcompPacket::from_bytes(buf)?;
                Response::Pubcomp {
                    packet_id: pubcomp.packet_id,
                }
            }
            _ => Response::Unknown,
        };
        Ok(packet)
    }
}

/// Abstracted Protocol that wraps a TcpStream and manages
/// sending & receiving of messages
pub struct Protocol {
    reader: io::BufReader<TcpStream>,
    stream: TcpStream,
}

impl Protocol {
    /// Wrap a TcpStream with Protocol
    pub fn with_stream(stream: TcpStream) -> io::Result<Self> {
        Ok(Self {
            reader: io::BufReader::new(stream.try_clone()?),
            stream,
        })
    }

    /// Establish a connection, wrap stream in BufReader/Writer
    pub fn connect(dest: SocketAddr) -> io::Result<Self> {
        let stream = TcpStream::connect(dest)?;
        eprintln!("Connecting to {}", dest);
        Self::with_stream(stream)
    }

    pub fn disconnect(&mut self) -> io::Result<()> {
        let disconnect_request = Request::Disconnect;
        self.send_message(&disconnect_request)
    }

    pub fn publish(&mut self, topic: &str, message: &[u8]) -> io::Result<()> {
        let pub_req = Request::Publish {
            packet_id: 1,
            qos: 1,
            topic: topic.to_string(),
            payload: message.to_vec(),
        };
        self.send_message(&pub_req)
    }

    pub fn ack(&mut self, ack_type: AckType) -> io::Result<()> {
        let ack_request = match ack_type {
            AckType::Puback(pkt_id) => Request::Puback { packet_id: pkt_id },
            AckType::Pubrec(pkt_id) => Request::Pubrec { packet_id: pkt_id },
            AckType::Pubrel(pkt_id) => Request::Pubrel { packet_id: pkt_id },
            AckType::Pubcomp(pkt_id) => Request::Pubcomp { packet_id: pkt_id },
        };
        self.send_message(&ack_request)
    }

    /// Serialize a message to the server and write it to the TcpStream
    pub fn send_message(&mut self, message: &impl Serialize) -> io::Result<()> {
        message.serialize(&mut self.stream)?;
        self.stream.flush()
    }

    /// Read a message from the inner TcpStream
    ///
    /// NOTE: Will block until there's data to read (or deserialize fails with io::ErrorKind::Interrupted)
    ///       so only use when a message is expected to arrive
    pub fn read_message<T: Deserialize>(&mut self) -> io::Result<T::Output> {
        T::deserialize(&mut self.reader)
    }
}

#[cfg(test)]
mod fixed_headers_tests {
    use super::*;

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
        let buf = &[0x10, 0x12, 0x04, b'M', b'Q', b'T', b'T', 0x04];
        let fixed_header = FixedHeader::from_bytes(&mut buf.as_slice()).unwrap();
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
        let mut buffer = vec![];
        fixed_header.write(&mut buffer).unwrap();
        assert_eq!(buffer, &[16, 18]);
    }
}
