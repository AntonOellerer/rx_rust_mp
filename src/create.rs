use crate::observable::Observable;
use crate::scheduler::Scheduler;
use std::marker::PhantomData;
use std::sync::mpsc::Sender;

pub struct Create<I, Item> {
    create_function: I,
    _marker: PhantomData<Item>,
}

pub fn create<I, Item>(create_function: I) -> Create<I, Item>
where
    I: Fn(Sender<std::io::Result<Item>>),
{
    Create {
        create_function,
        _marker: PhantomData::default(),
    }
}

impl<I, Item> Observable for Create<I, Item>
where
    I: Fn(Sender<std::io::Result<Item>>) + Send + 'static,
    Item: Send + 'static,
{
    type Item = Item;

    fn actual_subscribe<O>(self, channel: Sender<std::io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler,
    {
        pool.schedule(move || (self.create_function)(channel));
    }
}

#[cfg(test)]
mod test {
    use crate::create::create;
    use crate::observable::Observable;
    use crate::observer::Observer;
    use futures::executor::ThreadPool;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn it_creates() {
        let collector = AtomicI32::new(0);
        let pool = ThreadPool::new().unwrap();
        create(|sender| {
            sender.next(1).unwrap();
            sender.next(2).unwrap();
            sender.next(3).unwrap();
        })
        .subscribe(
            |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            pool,
        );
        assert_eq!(collector.into_inner(), 6);
    }
}
