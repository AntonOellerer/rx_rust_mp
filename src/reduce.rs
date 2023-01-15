use crate::observable::Observable;
use crate::scheduler::Scheduler;
use std::io;
use std::io::ErrorKind;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct ReduceOp<Source, CollectResult, ReduceFunction> {
    pub(crate) source: Source,
    pub(crate) collector: CollectResult,
    pub(crate) func: ReduceFunction,
}

impl<Source, CollectResult, ReduceFunction> Observable
    for ReduceOp<Source, CollectResult, ReduceFunction>
where
    Source: Observable,
    Source::Item: Send + 'static,
    ReduceFunction: Fn(CollectResult, Source::Item) -> CollectResult + Send + 'static,
    CollectResult: Send + 'static,
{
    type Item = CollectResult;

    fn actual_subscribe<O>(mut self, channel: Sender<std::io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<Source::Item>>();
        pool.schedule(move || loop {
            let message = incoming_rx.recv();
            match message {
                Ok(Ok(message)) => self.collector = (self.func)(self.collector, message),
                Ok(Err(e)) => {
                    eprintln!("Reduce, inner unwrap: {:?}", e.to_string());
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
