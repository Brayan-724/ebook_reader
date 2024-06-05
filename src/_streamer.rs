mod fifo;
mod utils;

use std::fs::File;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::thread;

use log::{debug, info, log_enabled, trace, warn};

use crate::config::EbookConfig;
use crate::renderizer::RESOLUTION;
use crate::streamer::fifo::open_fifo;
use crate::streamer::utils::log_stdout;
use crate::utils::{forward_pipe, forward_pipe_threadful};
use crate::{AUDIO_LOG, PREVIEW_LOG, VIDEO_LOG};

use self::fifo::create_fifo;

const VIDEO_PATH: &str = "/tmp/ebook-reader.video.fifo";
const AUDIO_PATH: &str = "/tmp/ebook-reader.audio.fifo";
const PREVIEW_PATH: &str = "/tmp/ebook-reader.preview.fifo";

#[rustfmt::skip]
const PIPE1_COMMON_FLAGS: &[&str] = &[
    // "-loglevel", "error",

    // PIPE VIDEO
    "-f",        "rawvideo",
    "-pix_fmt",  "rgb32",
    "-video_size",        RESOLUTION,
    "-i",        VIDEO_PATH,

    // PIPE OUTPUT
    "-c:v",      "libx264",
    "-preset",   "veryfast",
    "-maxrate",  "3000k",
    "-bufsize",  "6000k",
    "-pix_fmt",  "yuv420p",
    "-g",        "15",
    "-r",        "30",
    "-b:v",      "3000k",

    "-f",        "nut",
    "pipe:1"
];

#[rustfmt::skip]
const PIPE2_COMMON_FLAGS: &[&str] = &[
    // "-loglevel", "error",

    // PIPE AUDIO
    "-f",        "f32le",
    "-ar",       "11500",
    "-ac",       "2",
    "-i",        AUDIO_PATH,

    // PIPE VIDEO
    "-f",        "nut",
    "-i",        "-",

    // OUTPUT
    // "-c:v",      "libx264",
    // "-preset",   "veryfast",
    // "-maxrate",  "3000k",
    // "-bufsize",  "6000k",
    // "-pix_fmt",  "yuv420p",
    // "-g",        "15",
    // "-r",        "30",
    "-c:v",      "copy",
    "-c:a",      "aac",
    "-b:v",      "3000k",
    "-b:a",      "3000k",
    "-ar",       "44100", // -sample_rate
];

#[rustfmt::skip]
const NON_PREVIEW_FLAGS: &[&str] = &[
    "-f", "flv"
];

#[rustfmt::skip]
const PREVIEW_FLAGS: &[&str] = &[
    "-f", "tee",
    "-map", "1:v",
    "-map", "0:a",
];

const AUDIO_CAP: usize = 800;
const EMPTY_AUDIO: &[u8; AUDIO_CAP] = &[0; AUDIO_CAP];

pub struct TwitchStream {
    video_fifo: File,
    pub audio_fifo: File,
    video_buf: Vec<u8>,
    audio_buf_pointer: usize,
    audio_buf: Vec<u8>,
}

impl TwitchStream {
    pub fn new(config: EbookConfig) -> io::Result<Self> {
        let mut preview_out = Self::spawn_child(&config);

        let video_fifo = open_fifo(VIDEO_PATH);
        let audio_fifo = open_fifo(AUDIO_PATH);

        if config.preview {
            let preview_out = preview_out.take().unwrap();
            let mut preview = Command::new("ffplay")
                .arg("pipe:")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()?;

            let preview_in = preview.stdin.take().unwrap();

            let preview_err = preview.stderr.take().unwrap();
            log_stdout(config.stream_key, PREVIEW_LOG, 0, preview_err);

            forward_pipe_threadful(preview_out, preview_in);
        }

        Ok(Self {
            video_fifo,
            video_buf: Vec::new(),

            audio_fifo,
            audio_buf_pointer: 0,
            audio_buf: Vec::new(),
        })
    }

    #[inline(always)]
    fn spawn_child(config: &EbookConfig) -> Option<File> {
        create_fifo(VIDEO_PATH);
        create_fifo(AUDIO_PATH);
        if config.preview {
            create_fifo(PREVIEW_PATH);
        }

        let preview = config.preview;
        let stream_key = config.stream_key.to_owned();
        thread::spawn(move || {
            let (ffmpeg_1, mut ffmpeg_2) = {
                let mut ffmpeg_1 = Command::new("ffmpeg");
                ffmpeg_1.args(PIPE1_COMMON_FLAGS);
                ffmpeg_1.stdout(Stdio::piped());
                ffmpeg_1.stderr(Stdio::piped());
                let mut ffmpeg_1 = ffmpeg_1.spawn().unwrap();
                debug!(target: "video", "Spawned FFMPEG");

                let stderr = ffmpeg_1.stderr.take().unwrap();
                log_stdout(stream_key.clone(), VIDEO_LOG, 2, stderr);

                let ffmpeg_1 = ffmpeg_1.stdout.take().unwrap();

                (ffmpeg_1, Command::new("ffmpeg"))
            };
            ffmpeg_2.args(PIPE2_COMMON_FLAGS);
            let output = if preview {
                ffmpeg_2.args(PREVIEW_FLAGS);
                ffmpeg_2.stdout(Stdio::piped());

                format!("[f=nut]pipe:|[f=flv]rtmp://live.twitch.tv/app/{stream_key}")
            } else {
                ffmpeg_2.args(NON_PREVIEW_FLAGS);
                ffmpeg_2.stdout(Stdio::null());

                format!("rtmp://live.twitch.tv/app/{stream_key}")
            };
            ffmpeg_2.arg(&output);
            ffmpeg_2.stdin(Stdio::piped());
            ffmpeg_2.stderr(Stdio::piped());

            let mut ffmpeg_2 = ffmpeg_2.spawn().unwrap();
            debug!(target: "audio", "Spawned FFMPEG");

            if log_enabled!(log::Level::Debug) {
                let stderr = ffmpeg_2.stderr.take().unwrap();
                log_stdout(stream_key, AUDIO_LOG, 1, stderr);
            } else {
                info!("Using forward");
                let stderr = ffmpeg_2.stderr.take().unwrap();
                forward_pipe_threadful(stderr, io::stdout());
            }

            let ffmpeg_2_in = ffmpeg_2.stdin.take().unwrap();
            forward_pipe_threadful(ffmpeg_1, ffmpeg_2_in);

            if preview {
                let mut preview_fifo = open_fifo(PREVIEW_PATH);

                let mut ffmpeg_out = ffmpeg_2.stdout.take().unwrap();

                forward_pipe(&mut ffmpeg_out, &mut preview_fifo);
            } else {
                _ = ffmpeg_2.wait();
            }
        });

        if preview {
            trace!(target: PREVIEW_LOG, "Opening fifo {PREVIEW_PATH}");
            let out = std::fs::File::open(PREVIEW_PATH).unwrap();
            trace!(target: PREVIEW_LOG, "Opened fifo {PREVIEW_PATH}");

            Some(out)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn set_video_buffer(&mut self, buf: Vec<u8>) {
        self.video_buf = buf;
    }

    #[inline(always)]
    pub fn set_audio_buffer(&mut self, buf: Vec<u8>) {
        self.audio_buf = buf;
    }

    pub fn send_video_frame(&mut self) {
        if self.video_buf.is_empty() {
            warn!(target: VIDEO_LOG, "Skipping empty buffer");
            return;
        }
        trace!("Writing to fifo");
        self.video_fifo.write(&self.video_buf).unwrap();
        trace!("Written to fifo");
    }

    pub fn send_video_raw_frame(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            warn!(target: VIDEO_LOG, "Skipping empty buffer");
            return;
        }
        self.video_fifo.write(buf).unwrap();
    }

    pub fn send_audio_raw_frame(&mut self, buf: &[u8]) {
        if buf.len() < 4 {
            warn!(target: AUDIO_LOG, "Buffer should have at least 4 bytes");
            return;
        }
        self.audio_fifo.write_all(buf).unwrap();
    }

    /// Returns if is available to send another buffer
    pub fn send_audio_next_frame(&mut self) -> bool {
        if self.audio_buf.is_empty() {
            self.send_audio_raw_frame(EMPTY_AUDIO);
            return true;
        }

        let pointer_end = (self.audio_buf_pointer + AUDIO_CAP).min(self.audio_buf.len());

        let b = &self.audio_buf[self.audio_buf_pointer..pointer_end];
        self.audio_fifo.write_all(b).unwrap();

        self.audio_buf_pointer = pointer_end;

        if self.audio_buf_pointer >= self.audio_buf.len() {
            self.audio_buf = Vec::new();
            self.audio_buf_pointer = 0;
            true
        } else {
            false
        }
    }
}
