use crate::map::MapOp;
use crate::scheduler::Scheduler;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub trait Observable: Sized {
    type Item;

    fn map<F, B>(self, f: F) -> MapOp<Self, F>
    where
        F: FnOnce(Self::Item) -> B,
    {
        MapOp {
            source: self,
            func: f,
        }
    }

    fn subscribe<F, S>(self, f: F, scheduler: S)
    where
        F: FnOnce(Self::Item) + Clone,
        S: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel();
        self.actual_subscribe(incoming_tx, scheduler);
        loop {
            let message = incoming_rx.recv();
            match message {
                Ok(Ok(message)) => (f.clone())(message),
                Ok(Err(e)) => panic!("{}", e.to_string()),
                Err(_) => break, // Channel closed
            }
            // if let Ok(Ok(message)) = message {
            //     (f.clone())(message);
            // } else {
            //     return;
            // }
        }
    }

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static;
}
