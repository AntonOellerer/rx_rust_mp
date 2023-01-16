#![feature(drain_filter)]
extern crate core;

#[cfg(feature = "math")]
pub mod average;
pub mod create;
pub mod filter;
pub mod flatten;
pub mod from_iter;
pub mod group_by;
pub mod map;
pub mod observable;
pub mod observer;
pub mod reduce;
pub mod scheduler;
#[cfg(feature = "recurring")]
pub mod sliding_window;

#[cfg(test)]
mod tests {
    use crate::create::create;
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use crate::observer::Observer;
    use futures::executor::ThreadPool;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn it_maps() {
        let collector = AtomicI32::new(0);
        from_iter(0..10).map(|v| v + 1).subscribe(
            |v| {
                assert!(v > 0 && v < 11);
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        assert_eq!(collector.into_inner(), 55);
    }

    #[test]
    fn it_filters() {
        let collector = AtomicI32::new(0);
        from_iter(0..10).filter(|v| v % 2 == 0).subscribe(
            |v| {
                assert_eq!(v % 2, 0);
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        assert_eq!(collector.into_inner(), 20);
    }

    #[test]
    fn it_reduces() {
        let collector = AtomicI32::new(0);
        from_iter(0..10).reduce(0, |c, v| c + v).subscribe(
            |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            ThreadPool::new().unwrap(),
        );
        assert_eq!(collector.into_inner(), 45);
    }

    #[test]
    fn it_groups() {
        let collector = AtomicI32::new(0);
        let pool = ThreadPool::new().unwrap();
        let pool_c = pool.clone();
        from_iter(0..10).group_by(|v| *v).subscribe(
            |group| {
                let key = group.key;
                group.subscribe(|v| assert_eq!(v, key), pool_c.clone());
                collector.fetch_add(key, Ordering::Relaxed);
            },
            pool,
        );
        assert_eq!(collector.into_inner(), 45);
    }

    #[test]
    fn it_flattens() {
        let collector = AtomicI32::new(0);
        let pool = ThreadPool::new().unwrap();
        from_iter(0..10)
            .map(|_| from_iter(0..10))
            .flatten()
            .subscribe(
                |v| {
                    collector.fetch_add(v, Ordering::Relaxed);
                },
                pool,
            );
        assert_eq!(collector.into_inner(), 450);
    }

    #[test]
    fn it_groups_flattens() {
        let collector = AtomicI32::new(0);
        let pool = ThreadPool::new().unwrap();
        from_iter(0..10).group_by(|v| *v).flatten().subscribe(
            |v| {
                collector.fetch_add(v, Ordering::Relaxed);
            },
            pool,
        );
        assert_eq!(collector.into_inner(), 45);
    }

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

    #[test]
    fn it_flat_maps() {
        let collector = AtomicI32::new(0);
        let pool = ThreadPool::new().unwrap();
        from_iter(0..10)
            .flat_map(|v| create(move |s| s.next(v).unwrap()))
            .subscribe(
                |v| {
                    collector.fetch_add(v, Ordering::Relaxed);
                },
                pool,
            );
        assert_eq!(collector.into_inner(), 45);
    }

    #[test]
    fn it_averages() {
        let collector = Arc::new(RefCell::new(0_f64));
        let pool = ThreadPool::new().unwrap();
        from_iter(0..10).map(f64::from).average().subscribe(
            |v| {
                *collector.borrow_mut() += v;
            },
            pool,
        );
        assert_eq!(*collector.borrow(), 4.5_f64);
    }
}
