use crate::scheduler::Scheduler;
use std::sync::mpsc::{Receiver, Sender};

pub fn forward_messages<Item, O>(incoming: Receiver<Item>, outgoing: Sender<Item>, pool: O)
where
    Item: Send + 'static,
    O: Scheduler,
{
    pool.schedule(move || loop {
        let message = incoming.recv();
        match message {
            Ok(message) => outgoing.send(message).unwrap(),
            Err(_) => break, // Channel closed
        }
    })
    .forget();
}
