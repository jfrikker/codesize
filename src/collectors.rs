#![warn(clippy::correctness)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]

use std::cmp::Reverse;
use std::collections::{BinaryHeap, BTreeMap};
use std::collections::btree_map::{IntoIter as MapIntoIter};
use std::path::Path;

pub trait Collector {
    fn increment(&mut self, ext: String, path: &Path, count: u64);
    fn print_counts(self: Box<Self>, human_readable_base: Option<u64>);
}

#[derive(Default)]
struct PerExtension<D> {
    data: BTreeMap<String, D>
}

impl <D: Default> PerExtension<D> {
    fn increment(&mut self, ext: String, f: impl FnOnce(&mut D)) {
        f(self.data.entry(ext).or_insert_with(D::default));
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl <D> IntoIterator for PerExtension<D> {
    type Item = (String, D);
    type IntoIter = MapIntoIter<String, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[derive(Default)]
pub struct PerExtensionCount {
    count: PerExtension<u64>
}

impl Collector for PerExtensionCount {
    fn increment(&mut self, ext: String, _: &Path, count: u64) {
        self.count.increment(ext, |c| *c += count)
    }

    fn print_counts(self: Box<Self>, human_readable_base: Option<u64>) {
        if self.count.is_empty() {
            return;
        }

        let mut counts: Vec<(String, u64)> = self.count.into_iter().collect();
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
            println!("{} {}", ext, format_human_readable(count, human_readable_base))
        }
    }
}

#[allow(clippy::useless_let_if_seq)]
fn format_human_readable(mut num: u64, base: Option<u64>) -> String {
    base.map(|base| {
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
    })
    .unwrap_or_else(|| format!("{}", num))
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct WithSize<T> {
    size: u64,
    value: T
}

pub struct PerExtensionMax {
    queues: PerExtension<BinaryHeap<Reverse<WithSize<String>>>>,
    target: usize
}

impl PerExtensionMax {
    pub fn new(target: usize) -> Self {
        PerExtensionMax {
            queues: PerExtension::default(),
            target
        }
    }
}

impl Collector for PerExtensionMax {
    fn increment(&mut self, ext: String, path: &Path, count: u64) {
        let elem = WithSize {
            size: count,
            value: path.to_str().unwrap().to_string()
        };
        let target = self.target;
        self.queues.increment(ext, |heap| {
            heap.push(Reverse(elem));
            while heap.len() > target {
                heap.pop();
            }
        });
    }

    fn print_counts(self: Box<Self>, human_readable_base: Option<u64>) {
        for (ext, queue) in self.queues {
            println!("{}", ext);
            for Reverse(elem) in queue.into_sorted_vec() {
                println!("  {} {}", format_human_readable(elem.size, human_readable_base), elem.value);
            }
        }
    }
}
