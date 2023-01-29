use crate::observable::Observable;
use crate::scheduler::Scheduler;
use log::error;
use num_traits::Zero;
use std::io;
use std::io::ErrorKind;
use std::ops::{AddAssign, Div};
use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct AverageObservable<Source, C, CC> {
    pub(crate) source: Source,
    pub(crate) collector: C,
    pub(crate) count: CC,
}

impl<C, Source, CC> Observable for AverageObservable<Source, C, CC>
where
    Source: Observable,
    Source::Item: Send + 'static + Zero,
    C: AddAssign<Source::Item> + Div<Source::Item, Output = Source::Item> + Send + 'static,
    CC: Into<Source::Item> + AddAssign<i32> + Send + 'static,
{
    type Item = Source::Item;

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
                        self.collector += message;
                        self.count += 1;
                    }
                    Ok(Err(e)) => {
                        error!("Reduce, inner unwrap: {:?}", e.to_string());
                        channel
                            .send(Err(io::Error::new(ErrorKind::Other, e)))
                            .unwrap();
                        break;
                    }
                    Err(_) => break, // Channel closed
                }
            }
            channel
                .send(Ok(self.collector / self.count.into()))
                .unwrap();
        })
        .forget();
        self.source.actual_subscribe(incoming_tx, pool);
    }
}

#[cfg(test)]
mod test {
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use futures::executor::ThreadPool;
    use std::sync::{Arc, Mutex};

    #[test]
    fn it_averages() {
        let collector = Arc::new(Mutex::new(0_f64));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = from_iter(0..10).map(f64::from).average().subscribe(
            move |v| {
                *collector.lock().unwrap() += v;
            },
            pool,
        );
        futures::executor::block_on(handle);
        assert_eq!(*collector_c.lock().unwrap(), 4.5_f64);
    }
}
