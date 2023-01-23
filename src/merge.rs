use crate::observable::Observable;
use crate::scheduler::Scheduler;
use crate::utils;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub struct MergeObservable<Source1, Source2> {
    pub(crate) source1: Source1,
    pub(crate) source2: Source2,
}

impl<Source> Observable for MergeObservable<Source, Source>
where
    Source: Observable,
    Source::Item: Send + 'static,
{
    type Item = Source::Item;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<Source::Item>>();
        utils::forward_messages(incoming_rx, channel, pool.clone());
        self.source1
            .actual_subscribe(incoming_tx.clone(), pool.clone());
        self.source2.actual_subscribe(incoming_tx, pool);
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
    fn it_merges() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let handle = from_iter(0..5).merge(from_iter(5..10)).subscribe(
            move |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 45);
    }
}
