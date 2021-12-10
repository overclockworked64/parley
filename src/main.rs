use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

const NICK: &str = "toot";
const USER: &str = "tootz";
const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;

async fn send(writer: &mut BufWriter<WriteHalf<'_>>, message: &str) {
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
    let (rx, tx) = stream.split();
    let mut writer = BufWriter::new(tx);
    let mut reader = BufReader::new(rx);

    send(&mut writer, format!("NICK {}", NICK).as_str()).await;
    send(&mut writer, format!("USER {} 0 * :{}", USER, USER).as_str()).await;

    loop {
        buf.clear();

        if let Some(message) = recv_msg(&mut reader, &mut buf).await {
            println!("{}", message);

            if message.starts_with("PING") {
                let reply = message.replace("PING", "PONG");
                send(&mut writer, &reply).await;
            }
        }
    }
}
