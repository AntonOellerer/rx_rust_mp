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
    M: FnOnce(S::Item) -> Item + Clone + Send + 'static,
    Item: Send + 'static,
{
    type Item = Item;
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
                Ok(Ok(message)) => pool_cc.schedule(move || {
                    let out = (func_c)(message);
                    channel_c.send(io::Result::Ok(out)).unwrap();
                }),
                Ok(Err(e)) => {
                    eprintln!("Map, inner unwrap: {:?}", e.to_string());
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
