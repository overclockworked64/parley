const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;
const NICK: &str = "toot";
const USER: &str = "toot";
const REALNAME: &str = "Ronnie Regan";

#[tokio::main]
async fn main() {
    let mut stream = parley::connect(NETWORK, PORT, NICK, USER, REALNAME).await;
    let (rx, mut tx) = stream.split();

    parley::join(&mut tx, "##toottoot").await;
    parley::mainloop(rx, &mut tx).await;
}