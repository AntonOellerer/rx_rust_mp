extern crate core;

mod filter;
mod from_iter;
mod map;
mod observable;
mod reduce;
mod scheduler;

#[cfg(test)]
mod tests {
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use futures::executor::ThreadPool;
    use std::thread;
    use std::time::{Duration, SystemTime};

    #[test]
    fn it_maps() {
        from_iter(0..10)
            .map(|v| v + 1)
            .subscribe(|v| assert!(v > 0 && v < 11), ThreadPool::new().unwrap());
        thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn it_filters() {
        from_iter(0..10)
            .filter(|v| v % 2 == 0)
            .subscribe(|v| assert_eq!(v % 2, 0), ThreadPool::new().unwrap());
        thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn it_reduces() {
        from_iter(0..10)
            .reduce(|c, v| c + v, 0)
            .subscribe(|v| assert_eq!(v, 45), ThreadPool::new().unwrap());
        thread::sleep(Duration::from_secs(1));
    }
}
