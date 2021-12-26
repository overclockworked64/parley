use std::collections::HashMap;
use futures::future::{BoxFuture, FutureExt};


use futures::Future;
use itertools::Itertools;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

#[derive(Clone, PartialEq)]
struct User {
    nick: Option<String>,
    ident: Option<String>,
    vhost: Option<String>,
    is_server: bool,
}

impl User {
    fn new(
        nick: Option<String>,
        ident: Option<String>,
        vhost: Option<String>,
        is_server: bool,
    ) -> User {
        User {
            nick,
            ident,
            vhost,
            is_server,
        }
    }
}

struct CommanderOrder {
    command: String,
    parameters: Vec<String>,
}

struct Message {
    sender: Option<User>,
    command: String,
    parameters: Vec<String>,
}

impl Message {
    fn new(sender: Option<User>, command: String, parameters: Vec<String>) -> Message {
        Message {
            sender,
            command,
            parameters,
        }
    }
}

type AsyncCallback<'a, 'wh> = Box<dyn Fn(&'a mut WriteHalf<'wh>) -> BoxFuture<'wh, ()>>;

#[derive(Default)]
pub struct AsyncCallbacks<'a, 'wh>(HashMap<&'static str, AsyncCallback<'a, 'wh>>);

impl<'a, 'wh> AsyncCallbacks<'a, 'wh> {
    pub fn insert<F, Fut>(&mut self, k: &'static str, f: F)
    where
        F: Fn(&'a mut WriteHalf<'wh>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'wh + Send,
    {
        self.0.insert(k, Box::new(move |wh| f(wh).boxed()));
    }
}

pub async fn mainloop(
    rx: ReadHalf<'_>,
    tx: &mut WriteHalf<'_>,
    callbacks: AsyncCallbacks<'_, '_>,
) {
    let mut buf = vec![0u8; 8192];

    let mut reader = BufReader::new(rx);

    let commander = User::new(
        Some("xvm`".to_string()),
        Some("~xvm".to_string()),
        Some("user/xvm".to_string()),
        false,
    );

    loop {
        buf.clear();

        if let Some(message) = recv_msg(&mut reader, &mut buf).await {
            println!("{}", message);

            let msg = parse_msg(message.clone());

            if msg.sender.is_none() && msg.command == "PING" {
                let reply = message.replace("PING", "PONG");
                send(tx, &reply).await;
            }

            if msg.sender == Some(commander.clone()) {
                let CommanderOrder {
                    command,
                    parameters,
                } = parse_order(msg.parameters);

                for (trigger, callback) in callbacks.0.iter() {
                    if *trigger == command {
                        callback(tx).await;
                    }
                }

                // match command.as_str() {
                //     "!join" => join( tx, &parameters[0]).await,
                //     "!part" => part( tx, &parameters[0]).await,
                //     _ => unimplemented!(),
                // }
            }
        }
    }
}

pub async fn connect(
    network: &str,
    port: u16,
    nick: &str,
    user: &str,
    realname: &str,
) -> TcpStream {
    let mut stream = TcpStream::connect((network, port)).await.unwrap();

    let (_rx, mut tx) = stream.split();

    send(&mut tx, format!("NICK {}", nick).as_str()).await;
    send(&mut tx, format!("USER {} 0 * :{}", user, realname).as_str()).await;

    stream
}

pub async fn join(stream: &mut WriteHalf<'_>, channel: &str) {
    send(stream, format!("JOIN {}", channel).as_str()).await;
}

pub async fn part(stream: &mut WriteHalf<'_>, channel: &str) {
    send(stream, format!("PART {}", channel).as_str()).await;
}

pub async fn send(stream: &mut WriteHalf<'_>, message: &str) {
    let msg = format!("{}\r\n", message);
    if let Err(e) = stream.write(msg.as_bytes()).await {
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

fn parse_msg(message: String) -> Message {
    /*
    We either get something like:

        [1]: `:strontium.libera.chat NOTICE * :*** Checking Ident` or
        [2]: `:xvm`!~xvm@user/xvm PRIVMSG toot :!join ##toottoot` or
        [3]: `PING :iridium.libera.chat`

    Then we split on whitespace and we have, for example:

        [":strontium.libera.chat", "NOTICE", "*", ":***", "Checking", "Ident"]

    If the message starts with ':' (colon), it means it's either

        [1]: from the server (if it does not contain '!')
        [2]: a regular message from another user (if it contains '!')
    
    If the message does not start with a colon, it's a PING-like message.
    TODO: check what other messages do not start with a colon
    */
    let m = message
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect::<Vec<String>>();

    let (sender, command, parameters) = if message.starts_with(':') {
        // First parse the sender
        let sender = &m[0];

        let sender = if sender.contains('!') {
            // The message contains '!', so we attempt to parse the nick, ident, and vhost
            let s = sender
                .strip_prefix(':')
                .unwrap()
                .split('!')
                .map(|x| x.to_owned())
                .collect::<Vec<String>>();
            /*
            Now we have this:

                ["xvm`", "~xvm@user/xvm PRIVMSG toot :", "!join ##toottoot"]
            */
            let nick = &s[0];
            let (ident, vhost) = s[1].split('@').collect_tuple().unwrap();

            User::new(
                Some(nick.to_owned()),
                Some(ident.to_owned()),
                Some(vhost.to_owned()),
                false,
            )
        } else {
            // The message does not contain '!', meaning it's from the server
            User::new(None, None, None, true)
        };

        let command = &m[1];
        let parameters = &m[2..];

        (Some(sender), command.to_owned(), parameters.to_vec())
    } else {
        // PING-like message
        let sender = None;
        let command = &m[0];
        let parameters = &m[1..];

        (sender, command.to_owned(), parameters.to_vec())
    };

    Message::new(sender, command, parameters)
}

fn parse_order(message: Vec<String>) -> CommanderOrder {
    let command = message[1].strip_prefix(':').unwrap().to_owned();
    let parameters = &message[2..];

    CommanderOrder {
        command,
        parameters: parameters.to_vec(),
    }
}

