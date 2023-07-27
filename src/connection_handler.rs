use std::{
    sync::{Arc, Mutex, mpsc},
    net::{Shutdown, TcpStream},
    io::{self, BufReader, BufRead, Write},
    thread,
};
use core::time::Duration;

use crate::Event;

/// Each `ConnectionHandler` takes care of one client connection.
#[derive(Debug)]
pub struct ConnectionHandler {
    id: usize,
    nick: Arc<str>,
    stream: Arc<Mutex<BufReader<TcpStream>>>,
    tx: mpsc::Sender<Event>,
}

impl ConnectionHandler {
    /// Arguments:
    /// - `id`: passed with all `Event`s for identification.
    /// - `stream`: `TcpStream` of the client connection.
    /// - `tx`: `mpsc::Sender` for sending `Event`s to `MessageHandler`.
    pub fn new(id: usize, stream: TcpStream, tx: mpsc::Sender<Event>)
        -> Option<Self>
    {
        let mut stream = BufReader::new(stream);
        let nick = match Self::nick(&mut stream) {
            Ok(nick) => nick,
            Err(e) if e.kind() == io::ErrorKind::InvalidData =>
                return None,
            Err(_) => {
                stream.get_mut().shutdown(Shutdown::Both)
                    .expect("Couldn't shutdown socket");
                return None;
            }
        };

        stream.get_mut().set_nonblocking(true)
            .expect("Couldn't make socket nonblocking");

        Some(Self {
            id,
            nick,
            stream: Arc::new(Mutex::new(stream)),
            tx,
        })
    }

    /// Takes a mutable reference to a stream and runs the nickname prompt.
    /// Returns an `Err` on I/O failure.
    fn nick(stream: &mut BufReader<TcpStream>) -> std::io::Result<Arc<str>> {
        let mut buf = String::new();

        loop {
            write!(stream.get_mut(), "Please enter your nick.\n> ")?;
            stream.read_line(&mut buf)?;
            buf = buf.trim().to_ascii_lowercase();

            if buf.is_empty() {
                writeln!(stream.get_mut(), "Nick can't be empty.\n")?;
            } else if !buf.chars().all(|c| c.is_ascii_alphanumeric()) {
                writeln!(stream.get_mut(), "Nick must be alphanumeric.\n")?;
                buf.clear();
            } else {
                break;
            }
        }

        Ok(buf.into())
    }

    /// Sends a signal over `Self::tx`.
    /// Panics if the receiver is hung up.
    fn signal(&self, signal: Event) {
        self.tx.send(signal).expect("Couldn't send signal to MessageHandler.")
    }

    /// Check a message and send a `Event::Chat` if it's valid.
    fn message(&self, msg: &str) {
        if msg.chars().any(|c| c.is_control() && !c.is_whitespace()) {
            return;
        }

        self.signal(Event::Chat(
            self.id,
            msg.trim().into(),
        ));
    }

    /// Sends a `Event::Join` and continuously polls for new messages.
    /// Panics when the `Mutex` to the `TcpStream` couldn't be locked.
    /// Returns when the connection gets closed (EOF or I/O error).
    pub fn run(&mut self) {
        let timeout = Duration::from_millis(100);
        let mut buf = String::new();

        self.signal(Event::Join(
            self.id,
            Arc::clone(&self.nick),
            Arc::clone(&self.stream),
        ));

        loop {
            let res = self.stream.lock().unwrap().read_line(&mut buf);

            match res {
                // EOF
                Ok(0) => {
                    self.signal(Event::Disconnect(
                        self.id,
                        "Disconnected".into(),
                    ));
                    break;
                }
                // Bytes received.
                Ok(_) => {
                    self.message(&buf);
                    buf.clear()
                }
                // Nothing new.
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Wait before polling again.
                    thread::sleep(timeout);
                    continue;
                }
                // I/O failure / disconnect.
                Err(e) => {
                    self.signal(Event::Disconnect(
                        self.id,
                        e.kind().to_string(),
                    ));
                    break;
                }
            }
        }
    }
}
