use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const NICK: &str = "toot";
const USER: &str = "tootz";
const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;

const TERMINATOR_LENGTH: usize = 2;

async fn send(stream: &mut TcpStream, message: &str) {
    let msg = format!("{}\r\n", message);
    if let Err(e) = stream.write(msg.as_bytes()).await {
        eprintln!("writing to stream failed: {}", e);
    }
}

async fn recv_msg(stream: &mut TcpStream, buf: &mut [u8]) -> String {
    while !String::from_utf8(buf.to_vec()).unwrap().contains("\r\n") {
        let _ = stream.read(buf).await.unwrap();
    }

    let chunk = String::from_utf8(buf.to_vec()).unwrap();
    let terminator_index = chunk.find("\r\n").unwrap();
    let message = chunk
        .chars()
        .take(terminator_index + TERMINATOR_LENGTH)
        .filter(|x| *x != '\0')
        .collect::<String>();

    for i in 0..terminator_index + TERMINATOR_LENGTH {
        buf[i] = 0;
    }

    message
}

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect((NETWORK, PORT)).await.unwrap();

    send(&mut stream, format!("NICK {}", NICK).as_str()).await;
    send(&mut stream, format!("USER {} 0 * :{}", USER, USER).as_str()).await;

    let mut buf = [0u8; 2048];

    loop {
        let message = recv_msg(&mut stream, &mut buf).await;

        println!("{}", message);

        if message.starts_with("PING") {
            let reply = message.replace("PING", "PONG");
            send(&mut stream, &reply).await;
        }
    }
}
