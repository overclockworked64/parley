use futures::future::BoxFuture;
use std::collections::HashMap;

use itertools::Itertools;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

#[derive(Clone, PartialEq)]
pub struct User {
    nick: Option<String>,
    ident: Option<String>,
    vhost: Option<String>,
    is_server: bool,
}

impl User {
    pub fn new(
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
    sender: User,
    command: String,
    parameters: Vec<String>,
}

impl Message {
    fn new(sender: User, command: String, parameters: Vec<String>) -> Message {
        Message {
            sender,
            command,
            parameters,
        }
    }
}

pub struct Robot {
    tx: Option<OwnedWriteHalf>,
    reader: Option<BufReader<OwnedReadHalf>>,
    commander: User,
    callbacks: Option<AsyncCallbacks>,
}

impl Robot {
    pub fn new(commander: User, callbacks: Option<AsyncCallbacks>) -> Robot {
        Robot {
            tx: None,
            reader: None,
            commander,
            callbacks,
        }
    }

    pub async fn connect(
        &mut self,
        network: &str,
        port: u16,
        nick: &str,
        user: &str,
        realname: &str,
    ) {
        let stream = TcpStream::connect((network, port)).await.unwrap();

        let (rx, tx) = stream.into_split();
        let reader = BufReader::new(rx);

        self.tx = Some(tx);
        self.reader = Some(reader);

        self.send(format!("NICK {}", nick).as_str()).await;
        self.send(format!("USER {} 0 * :{}", user, realname).as_str())
            .await;
    }

    pub async fn join(&mut self, channel: &str) {
        self.send(format!("JOIN {}", channel).as_str()).await;
    }

    pub async fn part(&mut self, channel: &str) {
        self.send(format!("PART {}", channel).as_str()).await;
    }

    pub async fn mainloop(&mut self) {
        let mut buf = vec![0u8; 8192];

        loop {
            buf.clear();

            if let Some(message) = self.recv_msg(&mut buf).await {
                println!("{}", message);

                let msg = self.parse_msg(message.clone());

                if msg.sender.is_server && msg.command == "PING" {
                    let reply = message.replace("PING", "PONG");
                    self.send(&reply).await;
                }

                if msg.sender == self.commander {
                    let CommanderOrder {
                        command,
                        parameters,
                    } = self.parse_order(msg.parameters);

                    match &self.callbacks {
                        Some(callbacks) => {
                            if let Some(callback) = callbacks.get(command.as_str()) {
                                callback(self, parameters).await;
                            }
                        }
                        None => {}
                    }
                }
            }
        }
    }

    async fn send(&mut self, message: &str) {
        let msg = format!("{}\r\n", message);

        match self.tx.as_mut().unwrap().write(msg.as_bytes()).await {
            Ok(_) => {}
            Err(e) => eprintln!("writing to stream failed: {}", e),
        }
    }

    async fn recv_msg(&mut self, buf: &mut Vec<u8>) -> Option<String> {
        let msg = match self.reader.as_mut().unwrap().read_until(b'\n', buf).await {
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

    fn parse_msg(&self, message: String) -> Message {
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
            let sender = self.parse_sender(&m[0]);
            let command = &m[1];
            let parameters = &m[2..];

            (sender, command.to_owned(), parameters.to_vec())
        } else {
            // PING-like message
            let sender = User::new(None, None, None, true);
            let command = &m[0];
            let parameters = &m[1..];

            (sender, command.to_owned(), parameters.to_vec())
        };

        Message::new(sender, command, parameters)
    }

    fn parse_sender(&self, sender: &str) -> User {
        if sender.contains('!') {
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
        }
    }

    fn parse_order(&self, message: Vec<String>) -> CommanderOrder {
        let command = message[1].strip_prefix(':').unwrap().to_owned();
        let parameters = &message[2..];

        CommanderOrder {
            command,
            parameters: parameters.to_vec(),
        }
    }
}

#[derive(Default)]
pub struct AsyncCallbacks(HashMap<&'static str, AsyncCallback>);

impl AsyncCallbacks {
    pub fn insert(&mut self, k: &'static str, f: AsyncCallback) {
        self.0.insert(k, f);
    }

    pub fn get(&self, k: &str) -> Option<&AsyncCallback> {
        self.0.get(k)
    }
}

type AsyncCallback = fn(&mut Robot, Vec<String>) -> BoxFuture<'_, ()>;
