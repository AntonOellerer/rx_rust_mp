use log::{error, trace};
use std::io;
use std::io::ErrorKind;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

use crate::observable::Observable;
use crate::scheduler::Scheduler;
use crate::utils;

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
        utils::forward_messages(incoming_rx, channel, pool.clone());
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
        pool.schedule(move || {
            loop {
                let message = incoming_rx.recv();
                match message {
                    Ok(Ok(message)) => {
                        FlattenObserver { source: message }
                            .actual_subscribe(subscriber_tx.clone(), pool_c.clone());
                    }
                    Ok(Err(e)) => {
                        error!("Flatten: {:?}", e.to_string());
                        channel_c
                            .send(Err(io::Error::new(ErrorKind::Other, e)))
                            .unwrap();
                        break;
                    }
                    Err(_) => break, // Channel closed
                }
            }
            trace!("Flatten finished");
        })
        .forget();
        utils::forward_messages(subscriber_rx, channel, pool.clone());
        self.source.actual_subscribe(incoming_tx, pool);
    }
}

#[cfg(test)]
mod tests {
    use crate::create::create;
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use crate::observer::Observer;
    use futures::executor::ThreadPool;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn it_flattens() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = from_iter(0..10)
            .map(|_| from_iter(0..10))
            .flatten()
            .subscribe(
                move |v| {
                    collector.fetch_add(v, Ordering::Relaxed);
                },
                pool,
            );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 450);
    }

    #[test]
    fn it_groups_flattens() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = from_iter(0..10).group_by(|v| *v).flatten().subscribe(
            move |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            pool,
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 45);
    }

    #[test]
    fn it_flat_maps() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = from_iter(0..10)
            .flat_map(|v| create(move |s| s.next(v).unwrap()))
            .subscribe(
                move |v| {
                    collector.fetch_add(v, Ordering::Relaxed);
                },
                pool,
            );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 45);
    }
}
