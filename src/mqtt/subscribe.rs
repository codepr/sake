use crate::mqtt::{protocol, Qos};
use byteorder::{NetworkEndian, WriteBytesExt};
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct SubscriptionTopic {
    pub qos: Qos,
    pub topic: String,
}

#[derive(Debug)]
pub struct SubscribePacket {
    pub packet_id: u16,
    pub subscription_topics: Vec<SubscriptionTopic>,
}

impl SubscribePacket {
    pub fn new(packet_id: u16, subscription_topics: Vec<SubscriptionTopic>) -> Self {
        Self {
            packet_id,
            subscription_topics,
        }
    }

    pub fn write(&self, buf: &mut impl Write) -> io::Result<()> {
        buf.write_u16::<NetworkEndian>(self.packet_id)?;
        self.subscription_topics
            .iter()
            .for_each(|s: &SubscriptionTopic| {
                protocol::write_string(buf, &s.topic);
                buf.write_u8(s.qos as u8);
            });
        Ok(())
    }
}
