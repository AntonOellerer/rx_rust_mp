use std::io::{Error, ErrorKind};
use std::sync::mpsc::{SendError, Sender};

pub trait Observer {
    type Item;
    fn next(&self, value: Self::Item) -> Result<(), SendError<std::io::Result<Self::Item>>>;
    fn error(&self, err: Error) -> Result<(), SendError<std::io::Result<Self::Item>>>;
}

impl<Item> Observer for Sender<std::io::Result<Item>>
where
    Item: Send + 'static,
{
    type Item = Item;

    fn next(&self, value: Self::Item) -> Result<(), SendError<std::io::Result<Item>>> {
        self.send(Ok(value))
    }

    fn error(&self, err: Error) -> Result<(), SendError<std::io::Result<Item>>> {
        self.send(Err(Error::new(ErrorKind::Other, err)))
    }
}
