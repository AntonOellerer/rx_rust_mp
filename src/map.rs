use crate::observable::Observable;
use std::io;
use std::io::ErrorKind;

use crate::scheduler::Scheduler;

use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct MapOp<S, M> {
    pub(crate) source: S,
    pub(crate) func: M,
}

impl<Item, S, M> Observable for MapOp<S, M>
where
    S: Observable,
    S::Item: Send + 'static,
    M: Fn(S::Item) -> Item + Clone + Send + 'static,
    Item: Send + 'static,
{
    type Item = Item;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<S::Item>>();
        let pool_c = pool.clone();
        pool.schedule(move || loop {
            let message = incoming_rx.recv();
            let channel_c = channel.clone();
            let func_c = self.func.clone();
            let pool_cc = pool_c.clone();
            match message {
                Ok(Ok(message)) => {
                    pool_cc
                        .schedule(move || {
                            let out = (func_c)(message);
                            channel_c.send(Ok(out)).unwrap();
                        })
                        .forget();
                }
                Ok(Err(e)) => {
                    eprintln!("Map, inner unwrap: {:?}", e.to_string());
                    channel
                        .send(Err(io::Error::new(ErrorKind::Other, e)))
                        .unwrap();
                    break;
                }
                Err(_) => break, // Channel closed
            }
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
    fn it_maps() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let handle = from_iter(0..10).map(|v| v + 1).subscribe(
            move |v| {
                assert!(v > 0 && v < 11);
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 55);
    }
}
