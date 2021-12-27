use futures::future::FutureExt;

mod lib;

const NETWORK: &str = "irc.libera.chat";
const PORT: u16 = 6667;
const NICK: &str = "toot";
const USER: &str = "toot";
const REALNAME: &str = "Ronnie Regan";

async fn on_command(bot: &mut parley::Robot, params: Vec<String>) {
    bot.join(&params[0]).await;
}

#[tokio::main]
async fn main() {
    let mut bot = parley::Robot::new();

    let mut callbacks = parley::AsyncCallbacks::default();
    callbacks.insert("!join", |bot, param| on_command(bot, param).boxed());

    let commander = parley::User::new(
        Some("adder".to_string()),
        Some("~adder".to_string()),
        Some("user/adder".to_string()),
        false,
    );

    bot.connect(NETWORK, PORT, NICK, USER, REALNAME).await;
    bot.mainloop(callbacks, commander).await;
}
