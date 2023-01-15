use crate::observable::Observable;
use crate::scheduler::Scheduler;
use std::sync::mpsc::Sender;

pub struct FromIter<I> {
    iter: I,
}

pub fn from_iter<I>(iter: I) -> FromIter<I>
where
    I: IntoIterator,
{
    FromIter { iter }
}

impl<I> Observable for FromIter<I>
where
    I: IntoIterator,
    I::Item: Send + 'static,
{
    type Item = I::Item;

    fn actual_subscribe<O>(self, channel: Sender<std::io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler,
    {
        self.iter.into_iter().for_each(|v| {
            let channel_c = channel.clone();
            pool.schedule(move || channel_c.send(Ok(v)).unwrap())
        });
    }
}
