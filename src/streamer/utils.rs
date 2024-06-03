use std::io::{self, Read, Write};
use std::sync::{OnceLock, RwLock};
use std::thread;

use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ScrollUp};
use crossterm::{terminal, ExecutableCommand, QueueableCommand};
use log::{debug, log_enabled};

const MAX_BUFFERS: u16 = 3;
// static BUFFERS: RwLock<&[&[u8]; 3]> = RwLock::new(&[&[], &[], &[]]);

fn buffers() -> &'static RwLock<Vec<Vec<u8>>> {
    static BUFFERS: OnceLock<RwLock<Vec<Vec<u8>>>> = OnceLock::new();
    BUFFERS.get_or_init(|| RwLock::new(vec![vec![], vec![], vec![]]))
}

pub fn log_stdout<S>(stream_key: String, target: &'static str, buffer_idx: u16, mut stdout: S)
where
    S: Read + Send + 'static,
{
    if !log_enabled!(log::Level::Debug) {
        return;
    }
    thread::spawn(move || {
        let mut stdout_buffer = [0; 1024];
        loop {
            let n = stdout.read(&mut stdout_buffer).unwrap();

            let buf = &stdout_buffer[..n];
            send_log_buffer(&stream_key, target, buffer_idx as usize, buf);
        }
    });
}

pub fn send_log_buffer(stream_key: &str, target: &'static str, buffer_idx: usize, buf: &[u8]) {
    let mut log_buffers = buffers().write().unwrap();

    let term_size = terminal::size().unwrap();
    let buf_size = (term_size.0 as usize).min(buf.len());
    let buf_line = &buf[..buf_size];
    let old_log = log_buffers.get_mut(buffer_idx).unwrap();

    if old_log != buf_line {
        let msg = String::from_utf8_lossy(&buf);
        let msg = msg.trim();
        let msg = msg.replace(stream_key, "{REDACTED}");

        *old_log = buf_line
            .to_vec()
            .iter()
            .take_while(|c| **c != '\n' as u8)
            .copied()
            .collect();

        show_log(target, &msg, term_size.1, &log_buffers);
    }
}

pub fn show_log(target: &'static str, msg: &str, term_height: u16, log_buffers: &[Vec<u8>]) {
    let mut stdout = io::stdout();

    stdout
        .queue(MoveTo(0, term_height - MAX_BUFFERS))
        .unwrap()
        .execute(Clear(terminal::ClearType::CurrentLine))
        .unwrap();

    debug!(target: target, "{msg}");

    stdout.execute(ScrollUp(1)).unwrap();

    for (idx, b) in log_buffers.iter().enumerate() {
        stdout
            .queue(MoveTo(0, term_height - idx as u16 - 1))
            .unwrap()
            .queue(Clear(terminal::ClearType::CurrentLine))
            .unwrap();
        write!(stdout, "[{idx} {target}] ").unwrap();
        stdout.write(b).unwrap();
    }

    stdout.flush().unwrap();
}
