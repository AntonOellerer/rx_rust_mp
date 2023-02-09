use crate::observable::Observable;
use crate::scheduler::Scheduler;
use log::{error, trace};
use std::io;
use std::io::ErrorKind;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct SlidingWindowObservable<Source, Item, TimeFunction> {
    pub(crate) source: Source,
    pub(crate) interval: Duration,
    pub(crate) window_size: Duration,
    pub(crate) time_function: TimeFunction,
    pub(crate) buffer: Arc<Mutex<Vec<Item>>>,
}

impl<Source, TimeFunction> Observable
    for SlidingWindowObservable<Source, Source::Item, TimeFunction>
where
    Source: Observable,
    Source::Item: Send + 'static + Clone,
    TimeFunction: Fn(&Source::Item) -> Duration + Send + 'static + Clone,
{
    type Item = Vec<Source::Item>;

    fn actual_subscribe<O>(self, channel: Sender<io::Result<Self::Item>>, pool: O)
    where
        O: Scheduler + Clone + Send + 'static,
    {
        let (incoming_tx, incoming_rx) = mpsc::channel::<io::Result<Source::Item>>();
        let channel_c = channel.clone();
        let buffer_c = self.buffer.clone();
        let buffer_cc = self.buffer.clone();
        let time_function_c = self.time_function.clone();
        let handle = pool.schedule_repeating(
            move || {
                let mut unlocked_buffer = self.buffer.lock().unwrap();
                unlocked_buffer.drain_filter(|v| {
                    (self.time_function)(v) + self.window_size < get_now_duration()
                });
                let copied_buffer = unlocked_buffer.iter().cloned().collect();
                channel.send(Ok(copied_buffer)).unwrap();
            },
            self.interval,
        );
        pool.schedule(move || {
            loop {
                let message = incoming_rx.recv();
                match message {
                    Ok(Ok(message)) => buffer_c.lock().unwrap().push(message),
                    Ok(Err(e)) => {
                        error!("Sliding window, inner unwrap: {:?}", e.to_string());
                        channel_c
                            .send(Err(io::Error::new(ErrorKind::Other, e)))
                            .unwrap();
                        break;
                    }
                    Err(_) => {
                        let mut unlocked_buffer = buffer_cc.lock().unwrap();
                        unlocked_buffer.drain_filter(|v| {
                            (time_function_c)(v) + self.window_size < get_now_duration()
                        });
                        let copied_buffer = unlocked_buffer.iter().cloned().collect();
                        channel_c.send(Ok(copied_buffer)).unwrap();
                        handle.abort();
                        break;
                    } // Channel closed
                }
            }
            trace!("Sliding window finished");
        })
        .forget();
        self.source.actual_subscribe(incoming_tx, pool);
    }
}

pub fn get_now_duration() -> Duration {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Could not get epoch seconds")
}

#[cfg(test)]
mod tests {
    use crate::create::create;
    use crate::observable::Observable;
    use crate::observer::Observer;
    use crate::sliding_window::get_now_duration;
    use futures::executor::ThreadPool;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn it_buffers_in_sliding_windows() {
        let pool = ThreadPool::new().unwrap();

        let actual = Arc::new(Mutex::new(vec![]));
        let actual_c = actual.clone();

        let expected = vec![vec![0], vec![0, 1], vec![1], vec![1, 2], vec![2]];

        let handle = create(|sender| {
            let sleep = Duration::from_millis(43);
            sender.next(0).unwrap();
            std::thread::sleep(sleep);
            sender.next(1).unwrap();
            std::thread::sleep(sleep);
            sender.next(2).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        })
        .map(|v| (v, get_now_duration()))
        .sliding_window(Duration::from_millis(23), Duration::from_millis(59), |v| {
            v.1
        })
        .map(|window| window.iter().map(|(i, _)| *i).collect::<Vec<i32>>())
        .subscribe(move |buffer| actual.lock().unwrap().push(buffer), pool);

        futures::executor::block_on(handle);
        assert_eq!(expected, *actual_c.lock().unwrap());
    }
}
