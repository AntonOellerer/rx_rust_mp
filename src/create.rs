use crate::observable::Observable;
use crate::scheduler::Scheduler;
use std::marker::PhantomData;
use std::sync::mpsc::Sender;

pub struct Create<I, Item> {
    create_function: I,
    _marker: PhantomData<*const Item>,
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
