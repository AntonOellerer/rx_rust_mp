use futures::executor::ThreadPool;
use futures::future::{AbortHandle, RemoteHandle};
use futures::FutureExt;
use futures::StreamExt;
use std::time::Duration;

pub trait Scheduler {
    fn schedule(&self, task: impl FnOnce() + Send + 'static) -> RemoteHandle<()>;
    fn schedule_repeating<F>(&self, task: F, interval: Duration) -> AbortHandle
    where
        F: Fn() + Send + 'static;
}

impl Scheduler for ThreadPool {
    fn schedule(&self, task: impl FnOnce() + Send + 'static) -> RemoteHandle<()> {
        let future = async { (task)() };
        let (remote, remote_handle) = future.remote_handle();
        self.spawn_ok(remote);
        remote_handle
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
