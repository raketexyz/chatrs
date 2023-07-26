use std::{
    sync::{Arc, Mutex},
    io::BufReader,
    net::TcpStream,
};

mod message_handler;
mod connection_handler;

pub use message_handler::MessageHandler;
pub use connection_handler::ConnectionHandler;

#[derive(Debug, Clone)]
pub enum Event {
    Join(usize, Arc<str>, Arc<Mutex<BufReader<TcpStream>>>),
    Chat(usize, String),
    Disconnect(usize, String),
}
