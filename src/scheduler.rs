use futures::executor::ThreadPool;

pub trait Scheduler {
    fn schedule(&self, task: impl FnOnce() + Send + 'static);
}

impl Scheduler for ThreadPool {
    fn schedule(&self, task: impl FnOnce() + Send + 'static) {
        let future = async { (task)() };
        self.spawn_ok(future);
    }
}
