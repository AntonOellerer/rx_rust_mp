#[cfg(feature = "math")]
use crate::average::AverageObservable;
use crate::filter::FilterOp;
use crate::flatten::FlattenObservable;
use crate::group_by::{GroupByOp, SenderMap};
use crate::map::MapOp;
use crate::reduce::ReduceOp;
use crate::scheduler::Scheduler;
#[cfg(feature = "recurring")]
use crate::sliding_window::SlidingWindowObservable;
use futures::future::RemoteHandle;
use num_traits::Zero;
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

pub trait Observable: Sized {
    type Item;

    fn map<F, B>(self, f: F) -> MapOp<Self, F>
    where
        F: Fn(Self::Item) -> B,
    {
        MapOp {
            source: self,
            func: f,
        }
    }

    fn filter<F>(self, f: F) -> FilterOp<Self, F>
    where
        F: Fn(&Self::Item) -> bool,
    {
        FilterOp {
            source: self,
            func: f,
        }
    }

    fn reduce<C, R>(self, collector: C, f: R) -> ReduceOp<Self, C, R>
    where
        R: Fn(C, Self::Item) -> C,
    {
        ReduceOp {
            source: self,
            func: f,
            collector,
        }
    }

    fn group_by<GF, Key>(
        self,
        grouping_function: GF,
    ) -> GroupByOp<Self, GF, SenderMap<Key, Self::Item>>
    where
        GF: Fn(&Self::Item) -> Key,
    {
        GroupByOp {
            source: self,
            grouping_function,
            channel_store: HashMap::new(),
        }
    }

    fn flatten(self) -> FlattenObservable<Self> {
        FlattenObservable { source: self }
    }

    fn flat_map<F, B, Item>(self, f: F) -> FlattenObservable<MapOp<Self, F>>
    where
        F: Fn(Self::Item) -> B,
        B: Observable<Item = Item>,
    {
        FlattenObservable {
            source: MapOp {
                source: self,
                func: f,
            },
        }
    }

    #[cfg(feature = "math")]
    fn average(self) -> AverageObservable<Self, Self::Item, i32>
    where
        Self::Item: Zero,
    {
        AverageObservable {
            source: self,
            collector: Self::Item::zero(),
            count: 0,
        }
    }

    #[cfg(feature = "recurring")]
    fn sliding_window<F>(
        self,
        interval: Duration,
        window_size: Duration,
        time_function: F,
    ) -> SlidingWindowObservable<Self, Self::Item, F>
    where
        F: Fn(&Self::Item) -> Duration + Send + 'static,
    {
        SlidingWindowObservable {
            source: self,
            interval,
            window_size,
            time_function,
            buffer: Arc::new(Mutex::new(vec![])),
        }
    }

    fn subscribe<F, S>(self, mut f: F, scheduler: S) -> RemoteHandle<()>
    where
        F: FnMut(Self::Item) + Send + 'static,
        S: Scheduler + Clone + Send + 'static,
        Self::Item: Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel();
        let scheduler_c = scheduler.clone();
        self.actual_subscribe(incoming_tx, scheduler);
        scheduler_c.schedule(move || {
            loop {
                let message = incoming_rx.recv();
                match message {
                    Ok(Ok(message)) => (f)(message),
                    Ok(Err(e)) => panic!("{}", e.to_string()),
                    Err(_) => break, // Channel closed
                }
            }
        })
    }

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static;
}

#[cfg(test)]
mod tests {
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use futures::executor::ThreadPool;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn it_can_mut_access_external_state() {
        let collector = Arc::new(AtomicI32::new(0));
        let collector_c = collector.clone();
        let mut _test = 0;
        let handle = from_iter(0..10).subscribe(
            move |v| {
                let _test2 = &mut _test;
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        futures::executor::block_on(handle);
        assert_eq!(collector_c.load(Ordering::Relaxed), 45);
    }
}
