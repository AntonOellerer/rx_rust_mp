use crate::observable::Observable;
use crate::scheduler::Scheduler;
use log::trace;
use std::marker::PhantomData;
use std::sync::mpsc::Sender;

pub struct Create<I, Item> {
    create_function: I,
    _marker: PhantomData<Item>,
}

pub fn create<I, Item>(create_function: I) -> Create<I, Item>
where
    I: FnMut(Sender<std::io::Result<Item>>),
{
    Create {
        create_function,
        _marker: PhantomData::default(),
    }
}

impl<I, Item> Observable for Create<I, Item>
where
    I: FnMut(Sender<std::io::Result<Item>>) + Send + 'static,
    Item: Send + 'static,
{
    type Item = Item;

    fn actual_subscribe<O>(mut self, channel: Sender<std::io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler,
    {
        pool.schedule(move || {
            (self.create_function)(channel);
            trace!("Create finished");
        })
        .forget();
    }
}

#[cfg(test)]
mod test {
    use crate::create::create;
    use crate::observable::Observable;
    use crate::observer::Observer;
    use futures::executor::ThreadPool;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn it_creates() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let pool = ThreadPool::new().unwrap();
        let handle = create(|sender| {
            sender.next(1).unwrap();
            sender.next(2).unwrap();
            sender.next(3).unwrap();
        })
        .subscribe(
            move |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            pool,
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 6);
    }

    #[test]
    fn it_can_mut_access_external_state() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let mut _test = 0;
        let handle = create(move |sender| {
            sender.next(1).unwrap();
            sender.next(2).unwrap();
            sender.next(3).unwrap();
            let _test2 = &mut _test;
            assert_eq!(*_test2, 0);
        })
        .subscribe(
            move |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 6);
    }
}
