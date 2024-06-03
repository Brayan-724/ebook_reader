use std::ffi::CString;
use std::fs::{self, File};

use log::trace;

#[inline(always)]
pub fn open_fifo(path: &str) -> File {
    trace!(target: "fifo", "Opening {path}");
    let fifo = fs::OpenOptions::new().write(true).open(path).unwrap();
    trace!(target: "fifo", "Opened {path}");
    fifo
}

#[inline(always)]
pub fn create_fifo(path: &str) {
    trace!(target: "fifo", "Creating {path}");
    let filename = CString::new(path).unwrap();
    unsafe {
        libc::mkfifo(filename.as_ptr(), 0o600);
    }
    trace!(target: "fifo", "Created {path}");
}
