use std::io;
use std::io::ErrorKind;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

use crate::observable::Observable;
use crate::scheduler::Scheduler;

pub struct FlattenObserver<S> {
    pub(crate) source: S,
}

impl<S> FlattenObserver<S>
where
    S: Observable,
    S::Item: Send + 'static,
{
    fn actual_subscribe<O>(self, channel: Sender<io::Result<S::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<S::Item>>();
        pool.schedule(move || loop {
            let message = incoming_rx.recv();
            match message {
                Ok(message) => channel.send(message).unwrap(),
                Err(_) => break, // Channel closed
            }
        });
        self.source.actual_subscribe(incoming_tx, pool);
    }
}

pub struct FlattenObservable<S> {
    pub(crate) source: S,
}

impl<S> Observable for FlattenObservable<S>
where
    S: Observable,
    S::Item: Observable + Send + 'static,
    <S::Item as Observable>::Item: Send + 'static,
{
    type Item = <S::Item as Observable>::Item;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<S::Item>>();
        let (subscriber_tx, subscriber_rx) =
            mpsc::channel::<io::Result<<S::Item as Observable>::Item>>();
        let pool_c = pool.clone();
        let channel_c = channel.clone();
        pool.schedule(move || loop {
            let message = incoming_rx.recv();
            match message {
                Ok(Ok(message)) => {
                    FlattenObserver { source: message }
                        .actual_subscribe(subscriber_tx.clone(), pool_c.clone());
                }
                Ok(Err(e)) => {
                    eprintln!("Flatten: {:?}", e.to_string());
                    channel_c
                        .send(Err(io::Error::new(ErrorKind::Other, e)))
                        .unwrap();
                    break;
                }
                Err(_) => break, // Channel closed
            }
        });
        pool.schedule(move || loop {
            let message = subscriber_rx.recv();
            match message {
                Ok(message) => channel.send(message).unwrap(),
                Err(_) => break, // Channel closed
            }
        });
        self.source.actual_subscribe(incoming_tx, pool);
    }
}
