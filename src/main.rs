#[macro_use]
extern crate futures;

mod collectors;
mod walker;

use collectors::PerExtensionCountSender;
use clap::{Arg, App};
use futures::future::ok;
use futures::prelude::*;
use std::fmt::Debug;
use std::fs::Metadata;
use std::path::Path;
use tokio;
use tokio::fs::File;
use tokio::prelude::*;
use tokio::run;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CountType {
    Files,
    Bytes,
    Lines
}

fn main() {
    let matches = App::new("codesize")
                          .version("1.0")
                          .author("Joe Frikker <jfrikker@gmail.com>")
                          .about("Counts lines of code")
                          .arg(Arg::with_name("s")
                            .short("s")
                            .long("size")
                            .help("Sum file size"))
                          .arg(Arg::with_name("c")
                            .short("c")
                            .long("count")
                            .help("Count files"))
                          .arg(Arg::with_name("h")
                            .short("h")
                            .help("Output human-readable numbers"))
                            .get_matches();
    let count_type = if matches.is_present("s") {
        CountType::Bytes
    } else if matches.is_present("c") {
        CountType::Files
    } else {
        CountType::Lines
    };

    let human_readable = matches.is_present("h");

    let (fut, chan) = collectors::counter();

    let prog = ok(()).and_then(move |_| {
        let walker = walker::walk(Path::new(".").to_path_buf(),
            move |path, meta| count_file(count_type, path, meta, chan.clone()));
        spawn(walker);
        fut
    })
    .map(move |counts| {
        if !human_readable {
            print_counts(counts, None);
        } else if count_type == CountType::Bytes {
            print_counts(counts, Some(1024));
        } else {
            print_counts(counts, Some(1000));
        }
    })
    .map_err(|e| panic!("{}", e));
    run(prog);
}

fn spawn<F, E>(fut: F)
    where F: Future<Error=E> + Send + 'static,
          E: Debug {
    tokio::executor::spawn(fut.map(|_| ()).map_err(|e| panic!("{:?}", e)));
}

fn count_file(ctype: CountType, path: &Path, meta: &Metadata, chan: PerExtensionCountSender) {
    let ext = walker::extension(path);
    match ctype {
        CountType::Files => spawn(chan.send((ext, 1))),
        CountType::Bytes => spawn(chan.send((ext, meta.len()))),
        CountType::Lines => spawn(count_lines(path)
            .map_err(|e| panic!("{}", e))
            .then(|c| chan.send((ext, c.unwrap() as u64))))
    }
}

fn count_lines(path: &Path) -> impl Future<Item=usize, Error=std::io::Error> {
    File::open(path.to_owned())
        .and_then(|stream| {
            LineCounter {
                stream,
                buf: [0;102400],
                count: 0
            }
        })
}

struct LineCounter<R> {
    stream: R,
    buf: [u8;102400],
    count: usize
}

impl <R> Future for LineCounter<R>
    where R: AsyncRead {
    type Item = usize;
    type Error = std::io::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        #[allow(irrefutable_let_patterns)]
        while let count = try_ready!(self.stream.poll_read(&mut self.buf)) {
            if count == 0 {
                break;
            }

            for c in self.buf[0..count].iter() {
                if *c == 0x0A {
                    self.count += 1;
                }
            }
        }
        Ok(Async::Ready(self.count))
    }
}

fn print_counts(counts: collectors::PerExtensionCount, human_readable_base: Option<u64>) {
    if counts.is_empty() {
        return;
    }

    let mut counts: Vec<(String, u64)> = counts.into_iter().collect();
    counts.sort_unstable_by(|(ref ext1, ref count1), (ref ext2, ref count2)|
        count1.cmp(count2)
            .reverse()
            .then(ext1.cmp(ext2)));

    let mut max_len = counts.iter()
        .map(|(ref ext, _)| ext.len())
        .max()
        .unwrap();
    if max_len > 0 {
        max_len += 1;
    }

    counts.iter_mut().for_each(|(ref mut ext, _)| {
        if !ext.is_empty() {
            ext.insert_str(0, ".");
        }
        for _ in ext.len()..max_len {
            ext.push(' ');
        }
    });

    for (ext, count) in counts {
        if human_readable_base.is_none() {
            println!("{} {}", ext, count);
        } else {
            println!("{} {}", ext, format_human_readable(count, human_readable_base.unwrap()))
        }
    }
}

fn format_human_readable(mut num: u64, base: u64) -> String {
    let mut suffix = "";
    if num >= 10000 {
        num /= base;
        suffix = "K";
    }

    if num >= 10000 {
        num /= base;
        suffix = "M";
    }
    
    if num >= 10000 {
        num /= base;
        suffix = "G";
    }
    
    if num >= 10000 {
        num /= base;
        suffix = "T";
    }

    format!("{}{}", num, suffix)
}