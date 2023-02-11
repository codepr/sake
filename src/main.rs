use bytes::BytesMut;
use sake::mqtt::ConnackPacket;
use sake::mqtt::ConnectPacket;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    // Create a TcpStream
    let mut stream = TcpStream::connect("127.0.0.1:1883")?;
    let connect_packet = ConnectPacket::new("test".into(), false);
    // Try to write data, this may still fail with `WouldBlock`
    // if the readiness event is a false positive.
    let mut buf = BytesMut::new();
    match connect_packet.write(&mut buf) {
        Ok(_bytes) => println!("{}", stream.write(&buf)?),
        Err(_) => panic!("Unable to write"),
    }
    let mut buffer = [0; 128];
    stream.read(&mut buffer)?;
    match ConnackPacket::from_stream(buffer.iter()) {
        Ok(connack) => println!("{}", connack),
        Err(_) => panic!("Unable to read CONNACK"),
    }
    Ok(())
}
