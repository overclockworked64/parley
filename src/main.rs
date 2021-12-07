use std::net::{Ipv4Addr, SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect(SocketAddr::from((Ipv4Addr::new(64, 86, 243, 186), 6667)))
        .await
        .unwrap();

    stream.write(b"NICK toot\r\n").await.unwrap();
    stream.write(b"USER toot 0 * :toot\r\n").await.unwrap();

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

        for i in 0..terminator_index + 1 {
            buf[i] = 0u8;
        }

        println!("{:?}", message);

        if message.starts_with("PING") {
            let reply = message.replace("PING", "PONG");
            stream.write(reply.as_bytes()).await.unwrap();
        }
    }
}
