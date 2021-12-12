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

const COMMANDER: &str = "xvm`!~xvm@user/xvm";

async fn join(writer: &mut WriteHalf<'_>, channel: &str) {
    send(writer, format!("JOIN {}", channel).as_str()).await;    
}

async fn send(writer: &mut WriteHalf<'_>, message: &str) {
    let msg = format!("{}\r\n", message);
    if let Err(e) = writer.write(msg.as_bytes()).await {
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

    send(&mut tx, format!("NICK {}", NICK).as_str()).await;
    send(&mut tx, format!("USER {} 0 * :{}", USER, USER).as_str()).await;

    loop {
        buf.clear();

        if let Some(message) = recv_msg(&mut reader, &mut buf).await {
            println!("{}", message);

            if message.starts_with("PING") {
                let reply = message.replace("PING", "PONG");
                send(&mut tx, &reply).await;
            }

            if message.starts_with(format!(":{}", COMMANDER).as_str()) {
                let command = message.split(":").nth(2).unwrap();
                if command.starts_with("!join") {
                    let channel = command.split(" ").nth(1).unwrap();
                    join(&mut tx, channel).await;
                }
            }
        }
    }
}
