use futures::future::FutureExt;

mod lib;

const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;
const NICK: &str = "toot";
const USER: &str = "toot";
const REALNAME: &str = "Ronnie Regan";

async fn on_command(bot: &mut parley::Robot, param: String) {
    bot.join(&param).await;
}

#[tokio::main]
async fn main() {
    let mut bot = parley::Robot::new();

    let mut callbacks = parley::AsyncCallbacks::default();
    callbacks.insert("!join", |bot, param| on_command(bot, param).boxed());

    bot.connect(NETWORK, PORT, NICK, USER, REALNAME).await;
    bot.mainloop(callbacks).await;
}
