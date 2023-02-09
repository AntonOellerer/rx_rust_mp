use crate::observable::Observable;
use crate::scheduler::Scheduler;
use crate::utils;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct SubscribeOnObservable<Source, Pool> {
    pub(crate) source: Source,
    pub(crate) pool: Pool,
}

impl<Source, Pool> Observable for SubscribeOnObservable<Source, Pool>
where
    Source: Observable,
    Source::Item: Send + 'static,
    Pool: Scheduler + Clone + Send + 'static,
{
    type Item = Source::Item;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<Source::Item>>();
        utils::forward_messages(incoming_rx, channel, pool);
        self.source.actual_subscribe(incoming_tx, self.pool);
    }
}
