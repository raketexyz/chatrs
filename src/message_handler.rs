use std::{
    sync::{Arc, Mutex, mpsc},
    collections::HashMap,
    net::{Shutdown, TcpStream},
    io::{BufReader, Write},
};

use crate::Event;

pub struct MessageHandler {
    users: HashMap<usize, (Arc<str>, Arc<Mutex<BufReader<TcpStream>>>)>,
}

impl MessageHandler {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn start(&mut self, rx: mpsc::Receiver<Event>) {
        for event in rx.iter() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Join(id, nick, stream) => {
                if self.users.iter().any(|(_, (s, _))| s == &nick) {
                    if let Err(err) = writeln!(
                        stream.lock().unwrap().get_mut(),
                        "Already present."
                    ) {
                        eprintln!("{err}");
                    }
                    if let Err(err) = stream.lock().unwrap().get_mut()
                        .shutdown(Shutdown::Both)
                    {
                        eprintln!("{err}");
                    }
                    return;
                }

                self.broadcast(format!("[+] {nick}"));

                self.users.insert(id, (Arc::clone(&nick), stream));

                let (_, stream) = self.users.get(&id).unwrap();

                if let Err(e) = writeln!(
                    stream.lock().unwrap().get_mut(),
                    "online: {}",
                    self.users.iter().map(|(_, (nick, _))| nick.to_string())
                        .collect::<Vec<_>>().join(" "),
                ) {
                    eprintln!("While sending to {nick}: {e}");
                }
            }
            Event::Disconnect(id, reason) => {
                let nick = match self.users.get(&id) {
                    Some((nick, _)) => nick.clone(),
                    _ => return,
                };
                self.users.remove(&id);
                self.broadcast(format!("[-] {nick} ({reason})"));
            }
            Event::Chat(id, message) => {
                let (nick, _) = self.users.get(&id).unwrap();

                self.broadcast(format!("<{nick}> {message}"));
            }
        }
    }

    fn broadcast(&mut self, message: String) {
        println!("{message}");
        for (_id, (nick, stream)) in self.users.iter() {
            let res = writeln!(
                stream.lock().unwrap().get_mut(),
                "{message}",
            );

            if let Err(e) = res {
                eprintln!("While sending to {nick}: {e}");
            }
        }
    }
}

impl Default for MessageHandler {
    fn default() -> Self {
        Self::new()
    }
}
