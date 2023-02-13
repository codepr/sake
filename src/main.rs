use sake::mqtt::{Protocol, Request, Response};

fn main() -> std::io::Result<()> {
    // Create a TcpStream
    let request = Request::Connect {
        client_id: "test".into(),
        clean_session: false,
    };
    Protocol::connect("127.0.0.1:1883".parse().unwrap())
        .and_then(|mut client| {
            client.send_message(&request)?;
            Ok(client)
        })
        .and_then(|mut client| Ok((client.read_message::<Response>(), client)))
        .and_then(|(resp, client)| {
            println!("{}", resp?);
            Ok(client)
        })
        .and_then(|mut client| {
            let pub_req = Request::Publish {
                packet_id: 0,
                qos: 1,
                topic: "test-topic".into(),
                payload: b"test-payload".to_vec(),
            };
            client.send_message(&pub_req)?;
            Ok(client)
        })
        .and_then(|mut client| client.read_message::<Response>())
        .map(|resp| println!("{}", resp))?;

    Ok(())
}
