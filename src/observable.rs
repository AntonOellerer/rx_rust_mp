use crate::filter::FilterOp;
use crate::flatten::FlattenObservable;
use crate::group_by::{GroupByOp, SenderMap};
use crate::map::MapOp;
use crate::reduce::ReduceOp;
use crate::scheduler::Scheduler;
use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

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

    fn reduce<C, R>(self, f: R, collector: C) -> ReduceOp<Self, C, R>
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

    fn subscribe<F, S>(self, f: F, scheduler: S)
    where
        F: FnOnce(Self::Item) + Clone,
        S: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel();
        self.actual_subscribe(incoming_tx, scheduler);
        //todo this means that `subscribe` blocks until no more messages arrive, maybe move to different thread and return handle?
        loop {
            let message = incoming_rx.recv();
            match message {
                Ok(Ok(message)) => (f.clone())(message),
                Ok(Err(e)) => panic!("{}", e.to_string()),
                Err(_) => break, // Channel closed
            }
            // if let Ok(Ok(message)) = message {
            //     (f.clone())(message);
            // } else {
            //     return;
            // }
        }
    }

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static;
}
