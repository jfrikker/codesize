use std::collections::HashMap;
use std::collections::hash_map::{IntoIter as MapIntoIter};

#[derive(Default)]
struct PerExtension<D> {
    data: HashMap<String, D>
}

impl <D: Default> PerExtension<D> {
    fn increment(&mut self, ext: String, f: impl FnOnce(&mut D)) {
        f(self.data.entry(ext).or_insert(D::default()));
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

impl PerExtensionCount {
    pub fn increment(&mut self, ext: String, count: u64) {
        self.count.increment(ext, |c| *c += count)
    }


    pub fn print_counts(self, human_readable_base: Option<u64>) {
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
            if human_readable_base.is_none() {
                println!("{} {}", ext, count);
            } else {
                println!("{} {}", ext, format_human_readable(count, human_readable_base.unwrap()))
            }
        }
    }
}

#[allow(clippy::useless_let_if_seq)]
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

pub struct PerExtensionMax {
    queues: HashMap<String, u64>
}

