quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
        }
        Walkdir(err: walkdir::Error) {
            from()
        }
        Git(err: git2::Error) {
            from()
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;