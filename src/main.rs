use std::collections::HashMap;

use tokio::net::tcp::WriteHalf;

const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;
const NICK: &str = "toot";
const USER: &str = "toot";
const REALNAME: &str = "Ronnie Regan";

async fn callback(stream: &mut WriteHalf<'_>) {
    parley::join(stream, "##learnmath").await;
}

#[tokio::main]
async fn main() {
    let mut stream = parley::connect(NETWORK, PORT, NICK, USER, REALNAME).await;
    let (rx, mut tx) = stream.split();
    let mut callbacks = parley::AsyncCallbacks::default();
    callbacks.insert("!join", callback);


    parley::mainloop(rx, &mut tx, callbacks).await;
}