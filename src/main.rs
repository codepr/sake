use clap::ArgAction;
use clap::{arg, Command};
use sake::mqtt::{Protocol, Request, Response};

const DEFAULT_HOSTNAME: &str = "127.0.0.1";
const DEFAULT_CLIENT_ID: &str = "sake-cli";

fn cli() -> Command {
    Command::new("sake")
        .about("An MQTT utility CLI program")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("shell").about("Start an interactive MQTT shell"))
        .subcommand(
            Command::new("publish")
                .about("Publish a message to a topic")
                .arg(
                    arg!(--host <HOST>)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(ArgAction::Set)
                        .required(false),
                )
                .arg(
                    arg!(--message <MESSAGE>)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    arg!(--topic <TOPIC>)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    arg!(--client-id <CLIENT_ID>)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(ArgAction::Set)
                        .required(false),
                ),
        )
}

fn main() -> std::io::Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("shell", _)) => {
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
                .and_then(|(resp, mut client)| {
                    println!("{}", resp?);
                    let pub_req = Request::Publish {
                        packet_id: 1,
                        qos: 1,
                        topic: "test-topic".into(),
                        payload: b"test-payload".to_vec(),
                    };
                    client.send_message(&pub_req)?;
                    Ok(client)
                })
                .and_then(|mut client| Ok((client.read_message::<Response>(), client)))
                .and_then(|(resp, mut client)| {
                    println!("{}", resp?);
                    client.disconnect()
                })?;
        }
        Some(("publish", sub_matches)) => {
            let default_hostname = DEFAULT_HOSTNAME.to_string();
            let default_cid = DEFAULT_CLIENT_ID.to_string();
            let host = sub_matches
                .get_one::<String>("host")
                .unwrap_or(&default_hostname);
            let topic = sub_matches.get_one::<String>("topic").unwrap();
            let message = sub_matches.get_one::<String>("message").unwrap();
            let client_id = sub_matches
                .get_one::<String>("client_id")
                .unwrap_or(&default_cid);
            let request = Request::Connect {
                client_id: client_id.into(),
                clean_session: false,
            };
            Protocol::connect(format!("{}:1883", host).parse().unwrap())
                .and_then(|mut client| {
                    client.send_message(&request)?;
                    Ok(client)
                })
                .and_then(|mut client| Ok((client.read_message::<Response>(), client)))
                .and_then(|(resp, mut client)| {
                    println!("{}", resp?);
                    let pub_req = Request::Publish {
                        packet_id: 1,
                        qos: 1,
                        topic: topic.to_string(),
                        payload: message.as_bytes().to_vec(),
                    };
                    client.send_message(&pub_req)?;
                    Ok(client)
                })
                .and_then(|mut client| Ok((client.read_message::<Response>(), client)))
                .and_then(|(resp, mut client)| {
                    println!("{}", resp?);
                    client.disconnect()
                })?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
