use std::{
    net::TcpListener,
    thread,
    sync::mpsc,
};

use chatrs::{MessageHandler, ConnectionHandler};

mod config;

fn main() {
    let listener = TcpListener::bind(config::ADDRESS).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut handler = MessageHandler::new();

    // Start event handler in new thread.
    thread::spawn(move || {
        handler.start(rx);
    });

    println!("Listening on {}", config::ADDRESS);

    for (id, stream) in listener.incoming().map(|stream| stream.unwrap())
        .enumerate()
    {
        let tx = tx.clone();

        println!("Connection received on {}", stream.peer_addr().unwrap());

        // Spawn connection handler in new thread.
        thread::spawn(move || {
            if let Some(mut handler) = ConnectionHandler::new(id, stream, tx) {
                handler.run();
            }
        });
    }
}
