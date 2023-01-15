use crate::observable::Observable;
use crate::scheduler::Scheduler;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::io::ErrorKind;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

pub type SenderMap<Key, Item> = HashMap<Key, Sender<io::Result<Item>>>;

pub struct KeySubject<Key, Item> {
    pub(crate) key: Key,
    pub(crate) source: Receiver<io::Result<Item>>,
}

impl<Key, Item> Observable for KeySubject<Key, Item>
where
    Item: Send + 'static,
{
    type Item = Item;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        pool.schedule(move || loop {
            let message = self.source.recv();
            match message {
                Ok(Ok(message)) => {
                    channel.send(Ok(message)).unwrap();
                }
                Ok(Err(e)) => {
                    eprintln!("Group By, inner unwrap: {:?}", e.to_string());
                    channel
                        .send(Err(io::Error::new(ErrorKind::Other, e)))
                        .unwrap();
                    break;
                }
                Err(_) => break, // Channel closed
            }
        });
    }
}

pub struct GroupByOp<Source, GroupingFunction, CS> {
    pub(crate) source: Source,
    pub(crate) grouping_function: GroupingFunction,
    pub(crate) channel_store: CS,
}

impl<Source, GroupingFunction, Key> Observable
    for GroupByOp<Source, GroupingFunction, SenderMap<Key, Source::Item>>
where
    Source: Observable,
    Source::Item: Send + 'static,
    GroupingFunction: Fn(&Source::Item) -> Key + Send + 'static,
    Key: Send + 'static + Hash + Eq + Clone,
{
    type Item = KeySubject<Key, Source::Item>;

    fn actual_subscribe<O>(mut self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<Source::Item>>();
        pool.schedule(move || loop {
            let message = incoming_rx.recv();
            match message {
                Ok(Ok(message)) => {
                    let key = (self.grouping_function)(&message);
                    let sender = self.channel_store.entry(key.clone()).or_insert_with(|| {
                        let (subject_tx, subject_rx) = mpsc::channel::<io::Result<Source::Item>>();
                        channel
                            .send(Ok(KeySubject {
                                key,
                                source: subject_rx,
                            }))
                            .unwrap();
                        subject_tx
                    });
                    sender.send(Ok(message)).unwrap();
                }
                Ok(Err(e)) => {
                    eprintln!("Group By, inner unwrap: {:?}", e.to_string());
                    channel
                        .send(Err(io::Error::new(ErrorKind::Other, e)))
                        .unwrap();
                    break;
                }
                Err(_) => break, // Channel closed
            }
        });
        self.source.actual_subscribe(incoming_tx, pool);
    }
}
