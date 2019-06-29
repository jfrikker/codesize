use futures::future::{ok, poll_fn};
use std::fs::Metadata;
use std::sync::Arc;
use std::io::Error;
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;
use tokio::prelude::*;

pub fn walk<F>(base: PathBuf, visit: F) -> impl Future<Item=(), Error=Error> + Send + 'static
    where F: Fn(&Path, &Metadata) -> () + Send + Sync + 'static {
    walk_helper(base, Arc::new(visit))
}

fn walk_helper<F>(base: PathBuf, visit: Arc<F>) -> impl Future<Item=(), Error=Error> + Send + 'static
    where F: Fn(&Path, &Metadata) -> () + Send + Sync + 'static {
    read_dir(base)
        .flatten_stream()
        .for_each(move |entry| {
            let path = entry.path();
            let visit = visit.clone();
            poll_fn(move || entry.poll_metadata())
                .and_then(move |meta| {
                    let res: Box<Future<Item=(), Error=Error> + Send> = if meta.is_file() {
                        visit(&path, &meta);
                        Box::new(ok(()))
                    } else if meta.is_dir() {
                        Box::new(walk_helper(path, visit))
                    } else {
                        Box::new(ok(()))
                    };
                    res
                })
        })
}