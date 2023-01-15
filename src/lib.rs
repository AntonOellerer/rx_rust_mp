extern crate core;

mod filter;
mod from_iter;
mod map;
mod observable;
mod scheduler;

#[cfg(test)]
mod tests {
    use crate::from_iter::from_iter;
    use crate::observable::Observable;
    use futures::executor::ThreadPool;
    use std::thread;
    use std::time::{Duration, SystemTime};

    #[test]
    fn it_works() {
        from_iter(0..10)
            .map(|v| v + 1)
            .filter(|v| v % 2 == 0)
            .subscribe(
                |v| {
                    println!(
                        "Subscribe: value: {v}, thread: {:?}, time: {:?}",
                        thread::current().id(),
                        SystemTime::now()
                    )
                },
                ThreadPool::new().unwrap(),
            );
        thread::sleep(Duration::from_secs(10));
    }
}
