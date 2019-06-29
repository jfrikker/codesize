use crate::error::Result;
use nix::fcntl::OFlag;
use nix::sys::stat::{FileStat, Mode, SFlag};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};

pub fn walk(base: &Path, visitor: &mut impl Visitor) -> Result<()> {
    let fd = nix::fcntl::open(base, OFlag::O_DIRECTORY, Mode::empty())?;
    walk_helper(base, fd, visitor)
}

fn walk_helper(base_path: &Path, base_dir: RawFd, visitor: &mut impl Visitor) -> Result<()> {
    if !visitor.enter_directory(base_dir, base_path)? {
        return Ok(())
    }

    for entry in nix::dir::Dir::from_fd(base_dir)?.iter() {
        let entry = entry?;
        let name = entry.file_name();
        let fd = nix::fcntl::openat(base_dir, name, OFlag::empty(), Mode::empty())?;
        let stat = nix::sys::stat::fstat(fd)?;
        let ftype = SFlag::from_bits_truncate(stat.st_mode);
        if ftype == SFlag::S_IFREG {
            let mut path = PathBuf::from(base_path);
            path.push(OsStr::from_bytes(name.to_bytes()));
            visitor.visit(fd, &path, &stat)?;
        } else if ftype == SFlag::S_IFDIR && name.to_str().unwrap() != "." && name.to_str().unwrap() != ".." {
            let mut path = PathBuf::from(base_path);
            path.push(OsStr::from_bytes(name.to_bytes()));
            walk_helper(&path, fd, visitor)?;
        } else {
            nix::unistd::close(fd)?;
        }
    }
    visitor.leave_directory()?;

    Ok(())
}

pub trait Visitor {
    fn enter_directory(&mut self, fd: RawFd, path: &Path) -> Result<bool>;
    fn leave_directory(&mut self) -> Result<()>;
    fn visit(&mut self, fd: RawFd, path: &Path, stat: &FileStat) -> Result<()>;
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_string())
        .unwrap_or(String::new())
}