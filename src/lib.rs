use std::{
    sync::{Arc, Mutex},
    io::BufReader,
    net::TcpStream,
};

mod message_handler;
mod connection_handler;

pub use message_handler::MessageHandler;
pub use connection_handler::ConnectionHandler;

/// Events are sent from `ConnectionHandler`s to `MessageHandler`s via
/// `std::sync::mpsc`.
#[derive(Debug, Clone)]
pub enum Event {
    /// Sent when a user joins.
    Join(usize, Arc<str>, Arc<Mutex<BufReader<TcpStream>>>),
    /// Sent when a user submits a message.
    Chat(usize, String),
    /// Sent when a connection is closed.
    Disconnect(usize, String),
}
