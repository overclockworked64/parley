use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;

async fn send(stream: &mut TcpStream, message: &str) {
    stream
        .write(format!("{}\r\n", message).as_bytes())
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect((NETWORK, PORT)).await.unwrap();

    send(&mut stream, "NICK toot").await;
    send(&mut stream, "USER toot 0 * :toot").await;

    let mut buf = [0u8; 2048];

    loop {
        while !String::from_utf8(buf.to_vec()).unwrap().contains("\r\n") {
            let _ = stream.read(&mut buf).await.unwrap();
        }

        let chunk = String::from_utf8(buf.to_vec()).unwrap();
        let terminator_index = chunk.find("\r\n").unwrap();
        let message = chunk
            .chars()
            .take(terminator_index + 2)
            .filter(|x| *x != '\0')
            .collect::<String>();

        for i in 0..terminator_index + 2 {
            buf[i] = 0;
        }

        println!("{:?}", message);

        if message.starts_with("PING") {
            let reply = message.replace("PING", "PONG");
            send(&mut stream, &reply).await;
        }
    }
}
