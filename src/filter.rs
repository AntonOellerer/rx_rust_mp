use crate::observable::Observable;
use std::io;
use std::io::ErrorKind;

use crate::scheduler::Scheduler;

use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct FilterOp<S, F> {
    pub(crate) source: S,
    pub(crate) func: F,
}

impl<S, F> Observable for FilterOp<S, F>
where
    S: Observable,
    S::Item: Send + 'static,
    F: Fn(&S::Item) -> bool + Clone + Send + 'static,
{
    type Item = S::Item;
    // type Err = S::Err;

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
                            if (func_c)(&message) {
                                channel_c.send(Ok(message)).unwrap()
                            }
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
    fn it_filters() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let handle = from_iter(0..10).filter(|v| v % 2 == 0).subscribe(
            move |v| {
                assert_eq!(v % 2, 0);
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 20);
    }
}
