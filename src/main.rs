mod collectors;

use collectors::PerExtensionCount;
use clap::{Arg, App};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Result};
use std::path::Path;
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

    let mut counts = PerExtensionCount::new();

    for entry in WalkDir::new(base_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let ext = extension(entry.path());
            let count = match count_type {
                CountType::Files => 1,
                CountType::Bytes => entry.metadata()?.len(),
                CountType::Lines => count_lines(entry.path())?
            };
            counts.increment(ext, count);
        }
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