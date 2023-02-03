use crate::observable::Observable;
use crate::scheduler::Scheduler;
use crate::utils;
use log::{debug, error};
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::io::ErrorKind;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

pub type SenderMap<Key, Item> = HashMap<Key, Sender<io::Result<Item>>>;

pub struct KeySubject<Key, Item> {
    pub key: Key,
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
        utils::forward_messages(self.source, channel, pool);
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
        pool.schedule(move || {
            loop {
                let message = incoming_rx.recv();
                match message {
                    Ok(Ok(message)) => {
                        let key = (self.grouping_function)(&message);
                        let sender = self.channel_store.entry(key.clone()).or_insert_with(|| {
                            let (subject_tx, subject_rx) =
                                mpsc::channel::<io::Result<Source::Item>>();
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
                        error!("Group By, inner unwrap: {:?}", e.to_string());
                        channel
                            .send(Err(io::Error::new(ErrorKind::Other, e)))
                            .unwrap();
                        break;
                    }
                    Err(_) => break, // Channel closed
                }
            }
            debug!("Group by finished");
        })
        .forget();
        self.source.actual_subscribe(incoming_tx, pool);
    }
}

#[cfg(test)]
mod tests {
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use futures::executor::ThreadPool;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn it_groups() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = from_iter(0..10)
            .group_by(|v| *v)
            .map(|v_group| {
                let key = v_group.key;
                v_group.map(move |v| v * key)
            })
            .flatten()
            .subscribe(
                move |v| {
                    collector.fetch_add(v, Ordering::Relaxed);
                },
                pool,
            );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 285);
    }
}
