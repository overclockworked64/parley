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

async fn send(stream: &mut WriteHalf<'_>, message: &str) {
    let msg = format!("{}\r\n", message);
    if let Err(e) = stream.write(msg.as_bytes()).await {
        eprintln!("writing to stream failed: {}", e);
    }
}

async fn recv_msg(reader: &mut BufReader<ReadHalf<'_>>, buf: &mut Vec<u8>) -> Option<String> {
    if let Err(e) = reader.read_until(b'\n', buf).await {
        eprintln!("reading from stream failed: {}", e);

        None
    } else {
        let msg = String::from_utf8(buf.to_vec()).unwrap();

        Some(msg.trim().to_owned())
    }
}

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect((NETWORK, PORT)).await.unwrap();
    let (rx, mut tx) = stream.split();

    send(&mut tx, format!("NICK {}", NICK).as_str()).await;
    send(&mut tx, format!("USER {} 0 * :{}", USER, USER).as_str()).await;

    let mut buf = vec![0u8; 8192];
    let mut reader = BufReader::new(rx);

    loop {
        buf.clear();

        if let Some(message) = recv_msg(&mut reader, &mut buf).await {
            println!("{}", message);

            if message.starts_with("PING") {
                let reply = message.replace("PING", "PONG");
                send(&mut tx, &reply).await;
            }
        }
    }
}
