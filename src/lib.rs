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
pub mod merge;
pub mod observable;
pub mod observer;
pub mod reduce;
pub mod scheduler;
#[cfg(feature = "recurring")]
pub mod sliding_window;
pub mod utils;
