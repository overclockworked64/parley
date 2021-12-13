use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

const NICK: &str = "toot";
const USER: &str = "tootz";
const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;

struct Commander {
    nick: String,
    ident: String,
    vhost: String,
}

impl Commander {
    fn new(nick: String, ident: String, vhost: String) -> Commander {
        Commander { nick, ident, vhost }
    }

    fn to_string(&self) -> String {
        format!("{}!~{}@{}", self.nick, self.ident, self.vhost)
    }
}

struct CommanderOrder {
    command: String,
    parameter: String,
}

struct Message {
    sender: Option<String>,
    command: String,
    parameters: Vec<String>,
}

impl Message {
    fn new(sender: Option<String>, command: String, parameters: Vec<String>) -> Message {
        Message {
            sender,
            command,
            parameters,
        }
    }
}

fn parse_msg(message: String) -> Message {
    let m = message
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect::<Vec<String>>();

    let (sender, command, parameters) = if message.starts_with(':') {
        let sender = Some(
            m.clone()
                .into_iter()
                .nth(0)
                .unwrap()
                .chars()
                .skip(1)
                .collect(),
        );
        let command = m.clone().into_iter().nth(1).unwrap();
        let parameters = m.into_iter().skip(2).collect::<Vec<String>>();

        (sender, command, parameters)
    } else {
        let sender = None;
        let command = m.clone().into_iter().nth(0).unwrap();
        let parameters = m.into_iter().skip(1).collect::<Vec<String>>();

        (sender, command, parameters)
    };

    Message::new(sender, command, parameters)
}

fn parse_order(message: String) -> CommanderOrder {
    let msg = message.split(":").nth(2).unwrap().split_whitespace();
    let command = msg.clone().nth(0).unwrap().to_string();
    let parameter = msg.clone().nth(1).unwrap().to_string();

    CommanderOrder {
        command,
        parameter,
    }
}

async fn join(stream: &mut WriteHalf<'_>, channel: &str) {
    send(stream, format!("JOIN {}", channel).as_str()).await;
}

async fn part(stream: &mut WriteHalf<'_>, channel: &str) {
    send(stream, format!("PART {}", channel).as_str()).await;
}

async fn send(stream: &mut WriteHalf<'_>, message: &str) {
    let msg = format!("{}\r\n", message);
    if let Err(e) = stream.write(msg.as_bytes()).await {
        eprintln!("writing to stream failed: {}", e);
    }
}

async fn recv_msg(reader: &mut BufReader<ReadHalf<'_>>, buf: &mut Vec<u8>) -> Option<String> {
    let msg = match reader.read_until(b'\n', buf).await {
        Ok(_) => {
            let m = String::from_utf8(buf.to_vec()).unwrap();

            Some(m.trim().to_owned())
        }
        Err(e) => {
            eprintln!("reading from stream failed: {}", e);

            None
        }
    };
    msg
}

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect((NETWORK, PORT)).await.unwrap();
    let mut buf = vec![0u8; 8192];
    let (rx, mut tx) = stream.split();
    let mut reader = BufReader::new(rx);

    let commander = Commander::new(
        "xvm`".to_string(),
        "xvm".to_string(),
        "user/xvm".to_string(),
    );

    send(&mut tx, format!("NICK {}", NICK).as_str()).await;
    send(&mut tx, format!("USER {} 0 * :{}", USER, USER).as_str()).await;

    loop {
        buf.clear();

        if let Some(message) = recv_msg(&mut reader, &mut buf).await {
            println!("{}", message);

            let msg = parse_msg(message.clone());

            if msg.sender.is_none() && msg.command == "PING" {
                let reply = message.replace("PING", "PONG");
                send(&mut tx, &reply).await;
            }

            if msg.sender == Some(commander.to_string()) {
                let CommanderOrder { command, parameter } = parse_order(message.clone());

                match command.as_str() {
                    "!join" => join(&mut tx, &parameter).await,
                    "!part" => part(&mut tx, &parameter).await,
                    _ => unimplemented!(),
                }
            }
        }
    }
}
