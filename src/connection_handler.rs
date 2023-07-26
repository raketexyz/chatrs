use std::{
    sync::{Arc, Mutex, mpsc},
    net::{Shutdown, TcpStream},
    io::{self, BufReader, BufRead, Write},
    thread,
};
use core::time::Duration;

use crate::Event;

#[derive(Debug)]
pub struct ConnectionHandler {
    id: usize,
    nick: Arc<str>,
    stream: Arc<Mutex<BufReader<TcpStream>>>,
    tx: mpsc::Sender<Event>,
}

impl ConnectionHandler {
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

    fn signal(&self, signal: Event) {
        self.tx.send(signal).expect("Couldn't send signal to MessageHandler.")
    }

    pub fn run(&mut self) {
        let timeout = Duration::from_millis(100);

        self.signal(Event::Join(
            self.id,
            Arc::clone(&self.nick),
            Arc::clone(&self.stream),
        ));

        loop {
            let mut buf = String::new();
            let res = self.stream.lock().unwrap().read_line(&mut buf);

            match res {
                Ok(0) => {
                    self.signal(Event::Disconnect(
                        self.id,
                        "Disconnected".into(),
                    ));
                    break;
                }
                Ok(_) => {
                    if buf.chars().all(|c|
                        !c.is_ascii_control() || c.is_ascii_whitespace()
                    ) {
                        self.signal(Event::Chat(
                            self.id,
                            buf.trim().into(),
                        ));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(timeout);
                    continue;
                }
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                    self.signal(Event::Disconnect(
                        self.id,
                        "BrokenPipe".into(),
                    ));
                    break;
                }
                Err(e) => panic!("[Handler] {:?}", e),
            }
        }
    }
}
