use std::io::{Read, Write};
use std::thread;

use futures::channel::mpsc::UnboundedReceiver;

pub fn get_last_message<T>(rx: &mut UnboundedReceiver<T>) -> Option<T> {
    let mut last = None;

    while let Ok(Some(buf)) = rx.try_next() {
        last = Some(buf);
    }

    last
}

#[inline(always)]
pub fn forward_pipe<A: Read, B: Write>(a: &mut A, b: &mut B) {
    let mut buffer = [0; 1024];
    loop {
        let n = a.read(&mut buffer).unwrap();
        let n = b.write(&mut buffer[..n]).unwrap();
        if n > 0 {
            b.flush().unwrap();
        }
    }
}

#[inline(always)]
pub fn forward_pipe_threadful<A: Read + Send + 'static, B: Write + Send + 'static>(
    mut a: A,
    mut b: B,
) -> thread::JoinHandle<()> {
    thread::spawn(move || forward_pipe(&mut a, &mut b))
}
