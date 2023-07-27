use std::{
    sync::{Arc, Mutex, mpsc},
    collections::HashMap,
    net::{Shutdown, TcpStream},
    io::{BufReader, Write},
};

use crate::Event;

/// Handles `Event`s from all `ConnectionHandler`s.
pub struct MessageHandler {
    /// Maps the `id` of each `ConnectionHandler` to a tuple with the nick of
    /// the user and a reference (`Arc<Mutex<_>>`) to a `BufReader<..>` on the
    /// connected `TcpStream`.
    users: HashMap<usize, (Arc<str>, Arc<Mutex<BufReader<TcpStream>>>)>,
}

impl MessageHandler {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    /// Listens on the `mpsc::Receiver` and handles events in sequence.
    pub fn start(&mut self, rx: mpsc::Receiver<Event>) {
        for event in rx.iter() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Join(id, nick, stream) => self.join(id, nick, stream),
            Event::Disconnect(id, reason) => self.disconnect(id, reason),
            Event::Chat(id, message) => {
                let nick = match self.users.get(&id) {
                    Some((nick, _)) => nick.clone(),
                    _ => return,
                };

                self.broadcast(format!("<{nick}> {message}"));
            }
        }
    }

    /// Removes the `id` from the `users` `HashMap` and broadcasts a disconnect
    /// message.
    fn disconnect(&mut self, id: usize, reason: String) {
        let nick = match self.users.get(&id) {
            Some((nick, _)) => nick.clone(),
            _ => return,
        };

        self.users.remove(&id);
        self.broadcast(format!("[-] {nick} ({reason})"));
    }

    /// Checks if the nick is taken.
    /// Broadcasts a join message.
    /// Adds the new user to the `users` `HashMap` and sends a user list to the
    /// client.
    fn join(
        &mut self,
        id: usize,
        nick: Arc<str>,
        stream: Arc<Mutex<BufReader<TcpStream>>>,
    ) {
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

        let users = self.user_list();
        let (_, stream) = self.users.get(&id).unwrap();
        let res = writeln!(
            stream.lock().unwrap().get_mut(),
            "online: {users}",
        );

        if let Err(err) = res {
            eprintln!("While sending to {nick}: {err}");
        }
    }

    /// Returns a space-separated list of all users online.
    fn user_list(&mut self) -> String {
        self.users.iter().map(|(_, (nick, _))| nick.to_owned())
            .collect::<Vec<_>>().join(" ")
    }

    /// Prints `message` to `stdout` and sends it to all connected clients.
    fn broadcast(&mut self, message: String) {
        println!("{message}");

        for (nick, stream) in self.users.values() {
            let res = writeln!(
                stream.lock().unwrap().get_mut(),
                "{message}",
            );

            if let Err(err) = res {
                eprintln!("While sending to {nick}: {err}");
            }
        }
    }
}

impl Default for MessageHandler {
    fn default() -> Self {
        Self::new()
    }
}
