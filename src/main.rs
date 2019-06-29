#[macro_use]
extern crate quick_error;

mod collectors;
mod error;
mod walker;

use collectors::PerExtensionCount;
use clap::{Arg, App};
use error::Result;
use nix::sys::stat::FileStat;
use std::fmt::Debug;
use std::fs::Metadata;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, RawFd};
use std::path::Path;
use walker::{Visitor, extension, walk};

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
                          .arg(Arg::with_name("DIRECTORY")
                            .help("Base directory")
                            .index(1))
                            .get_matches();
    let count_type = if matches.is_present("s") {
        CountType::Bytes
    } else if matches.is_present("c") {
        CountType::Files
    } else {
        CountType::Lines
    };

    let human_readable = matches.is_present("h");
    let base_dir = matches.value_of("DIRECTORY").unwrap_or(".").to_owned();

    let mut visitor = Counter {
        counts: PerExtensionCount::new(),
        count_type
    };
    walker::walk(&Path::new(&base_dir), &mut visitor).unwrap();

    if !human_readable {
        visitor.counts.print_counts(None);
    } else if count_type == CountType::Bytes {
        visitor.counts.print_counts(Some(1024));
    } else {
        visitor.counts.print_counts(Some(1000));
    }
}

struct Counter {
    counts: PerExtensionCount,
    count_type: CountType
}

impl Visitor for Counter {
    fn enter_directory(&mut self, fd: RawFd, path: &Path) -> Result<bool> {
        Ok(true)
    }

    fn leave_directory(&mut self) -> Result<()> {
        Ok(())
    }

    fn visit(&mut self, fd: RawFd, path: &Path, stat: &FileStat) -> Result<()> {
        let ext = walker::extension(path);
        let count = match self.count_type {
            CountType::Files =>  {
                nix::unistd::close(fd)?;
                1
            },
            CountType::Bytes => {
                nix::unistd::close(fd)?;
                stat.st_size as u64
            },
            CountType::Lines => count_lines(fd)?
        };
        self.counts.increment(ext, count);
        Ok(())
    }
}

fn count_lines(fd: RawFd) -> Result<u64> {
    let mut file = unsafe { File::from_raw_fd(fd) };
    let mut buf = [0;102400];
    let mut result = 0;

    #[allow(irrefutable_let_patterns)]
    while let count = file.read(&mut buf)? {
        if count == 0 {
            break;
        }

        for c in buf[0..count].iter() {
            if *c == 0x0A {
                result += 1;
            }
        }
    }
    Ok(result)
}