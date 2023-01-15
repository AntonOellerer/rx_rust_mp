use crate::observable::Observable;
use crate::scheduler::Scheduler;
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
                        eprintln!("Reduce, inner unwrap: {:?}", e.to_string());
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
        });
        self.source.actual_subscribe(incoming_tx, pool);
    }
}