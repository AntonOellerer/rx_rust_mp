use futures::executor::ThreadPool;
use futures::future::AbortHandle;
use futures::FutureExt;
use futures::StreamExt;
use std::time::Duration;

pub trait Scheduler {
    fn schedule(&self, task: impl FnOnce() + Send + 'static);
    fn schedule_repeating<F>(&self, task: F, interval: Duration) -> AbortHandle
    where
        F: Fn() + Send + 'static;
}

impl Scheduler for ThreadPool {
    fn schedule(&self, task: impl FnOnce() + Send + 'static) {
        let future = async { (task)() };
        self.spawn_ok(future);
    }

    fn schedule_repeating<F>(&self, task: F, interval: Duration) -> AbortHandle
    where
        F: Fn() + Send + 'static,
    {
        let abortable = futures::future::abortable(futures::future::ready(()).then(move |_| {
            async_std::stream::interval(interval).for_each(move |_| {
                (task)();
                futures::future::ready(())
            })
        }));
        self.spawn_ok(abortable.0.map(|_| ()));
        abortable.1
    }
}
