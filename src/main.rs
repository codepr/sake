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
        .and_then(|mut client| client.read_message::<Response>())
        .map(|resp| println!("{}", resp))?;
    Ok(())
}
