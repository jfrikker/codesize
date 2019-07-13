#![warn(clippy::correctness)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]

#[macro_use] extern crate quick_error;

mod collectors;
mod error;

use collectors::{Collector, PerExtensionCount, PerExtensionMax};
use clap::{Arg, App};
use error::Result;
use git2::Repository;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CountType {
    Files,
    Bytes,
    Lines
}

fn main() -> Result<()> {
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
                          .arg(Arg::with_name("git")
                            .long("git")
                            .help("Only look at files in the git index"))
                          .arg(Arg::with_name("largest_count")
                            .short("l")
                            .long("largest")
                            .takes_value(true)
                            .help("Output the largest files per type"))
                          .arg(Arg::with_name("ext")
                            .long("ext")
                            .takes_value(true)
                            .multiple(true)
                            .number_of_values(1)
                            .help("Output the largest files per type"))
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
    let use_git = matches.is_present("git");
    let base_dir = matches.value_of("DIRECTORY").unwrap_or(".").to_owned();
    let largest: Option<usize> = matches.value_of("largest_count").map(|s| s.parse().unwrap());
    let extensions: Vec<&str> = matches.values_of("ext").map(Iterator::collect).unwrap_or_else(Vec::default);

    let mut counts: Box<dyn Collector> = largest
        .map(|count| {
            let res: Box<dyn Collector> = Box::new(PerExtensionMax::new(count));
            res
        })
        .unwrap_or_else(|| {
            let res: Box<dyn Collector> = Box::new(PerExtensionCount::default());
            res
        });

    if use_git {
        walk_git(&base_dir, count_type, extensions, counts.as_mut())?;
    } else {
        walk_normal(&base_dir, count_type, extensions, counts.as_mut())?;
    }

    if !human_readable {
        counts.print_counts(None);
    } else if count_type == CountType::Bytes {
        counts.print_counts(Some(1024));
    } else {
        counts.print_counts(Some(1000));
    }

    Ok(())
}

fn walk_normal<P: AsRef<Path>>(base_dir: P, count_type: CountType, 
                               extension_filter: Vec<&str>,
                               counts: &mut dyn Collector) -> Result<()> {
    for entry in WalkDir::new(base_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let ext = extension(entry.path());
            if !extension_filter.is_empty() && !extension_filter.contains(&(ext.as_ref())) {
                continue;
            }
            let count = match count_type {
                CountType::Files => 1,
                CountType::Bytes => entry.metadata()?.len(),
                CountType::Lines => count_lines(entry.path())?
            };
            counts.increment(ext, entry.path(), count);
        }
    }

    Ok(())
}

fn walk_git<P: AsRef<Path>>(base_dir: P, count_type: CountType,
                            extension_filter: Vec<&str>,
                            counts: &mut dyn Collector) -> Result<()> {
    let repo = Repository::open(&base_dir)?;
    for entry in repo.index()?.iter() {
        let path = Path::new(OsStr::from_bytes(&entry.path));
        let ext = extension(path);
        if !extension_filter.is_empty() && !extension_filter.contains(&(ext.as_ref())) {
            continue;
        }
        let count = match count_type {
            CountType::Files => 1,
            CountType::Bytes => u64::from(entry.file_size),
            CountType::Lines => {
                let mut full_path = PathBuf::new();
                full_path.push(&base_dir);
                full_path.push(path);
                count_lines(&full_path)?
            }
        };
        counts.increment(ext, path, count);
    }

    Ok(())
}

fn count_lines(path: &Path) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut buf = [0;102_400];
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

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(OsStr::to_str)
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}
