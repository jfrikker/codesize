mod collectors;
mod walker;

use futures::prelude::*;
use std::path::Path;
use tokio::run;

fn main() {
    run(walker::walk(Path::new(".").to_path_buf(), |path, _| println!("{:?}", path)).map_err(|e| panic!("{}", e)));
}
