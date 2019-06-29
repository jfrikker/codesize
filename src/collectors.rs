use futures::future::ok;
use std::collections::HashMap;
use tokio::prelude::*;
use tokio::sync::mpsc::{Sender, channel};
use tokio::sync::mpsc::error::RecvError;

pub type PerExtensionCount = HashMap<String, u64>;
pub type PerExtensionCountSender = Sender<(String, u64)>;

pub fn counter() -> (impl Future<Item=PerExtensionCount, Error=RecvError>, PerExtensionCountSender) {
    let (sender, receiver) = channel(16);
    let counts = HashMap::new();

    let fut = receiver.fold(
        counts,
        |mut c, (extension, count)| {
            *c.entry(extension).or_insert(0) += count;
            ok(c)
    });

    (fut, sender)
}