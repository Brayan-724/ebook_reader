#[cfg(feature = "hot-reload")]
#[hot_lib_reloader::hot_module(dylib = "lib")]
pub mod hot_lib {
    use flo_canvas::Draw;
    pub use lib::EbookContext;

    hot_functions_from_file!("lib/src/lib.rs");
}

#[cfg(not(feature = "hot-reload"))]
pub mod hot_lib {
    pub use lib::*;
}

pub mod config;
pub mod error;
mod logger;
mod render;
mod renderizer;
mod streamer;
mod tts;
mod utils;

use error::EbookResult;
use flo_render::initialize_offscreen_rendering;
use log::{error, info, trace, warn};
use streamer::TwitchStream;

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::config::EbookConfig;
use crate::logger::Logger;
use crate::renderizer::Renderizer;
use crate::tts::{mp3_to_wav, Languages};

pub const PREVIEW_LOG: &str = "\x1b[1;36mPREVIEW\x1b[0m";
pub const VIDEO_LOG: &str = "\x1b[1;35mVIDEO\x1b[0m";
pub const AUDIO_LOG: &str = "\x1b[1;34mAUDIO\x1b[0m";

fn main() {
    if let Err(err) = run() {
        if !log::log_enabled!(log::Level::Error) {
            env_logger::init();
        }

        error!("{err}");
    }
}

fn run() -> EbookResult<()> {
    let config = EbookConfig::from_envs()?;
    println!("{config}");

    _ = Logger::init(&config);

    let mut stream = TwitchStream::new(config).unwrap();
    let (video_tick_tx, video_tick_rx) = mpsc::channel::<()>();
    let (video_tx, video_rx) = mpsc::channel::<Vec<u8>>();
    let (audio_tick_tx, audio_tick_rx) = mpsc::channel::<()>();
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let tts = tts::TTS::new(Languages::Spanish, None);

        info!(target: AUDIO_LOG, "New Audio");
        // Sample Rate: 22050
        let audio_buf = tts.generate_audio("This is a test").unwrap();

        let mut hint = symphonia::core::probe::Hint::new();
        hint.with_extension("mp3");
        let wav = mp3_to_wav(&hint, audio_buf.clone());

        loop {
            info!(target: AUDIO_LOG, "Sending New Audio");
            audio_tx.send(wav.clone()).unwrap();
            std::thread::sleep(Duration::from_secs(1));

            audio_tick_rx.recv().unwrap();
        }
    });

    thread::spawn(move || {
        let render_context = initialize_offscreen_rendering().unwrap();
        let mut renderizer = Renderizer::new(render_context);

        loop {
            let b = renderizer.render();
            video_tx.send(b).unwrap();

            video_tick_rx.recv().unwrap();
        }
    });

    // video_tick_tx.send(()).unwrap();
    // audio_tick_tx.send(()).unwrap();

    // let mut audio_file = std::fs::File::create("./audio.wav").unwrap();
    // audio_file.;
    // audio_file.write(&[]).unwrap();
    let fps = Duration::from_millis(10);
    loop {
        info!("TICK");

        // Render
        if let Ok(b) = video_rx.try_recv() {
            trace!(target: VIDEO_LOG, "Buffer received");
            stream.set_video_buffer(b);

            video_tick_tx.send(()).unwrap();
        }

        if let Ok(b) = audio_rx.try_recv() {
            trace!(target: AUDIO_LOG, "Buffer received");
            // stream.set_audio_buffer(b);
            stream.send_audio_raw_frame(&b);
            trace!(target: AUDIO_LOG, "Buffer written");
            audio_tick_tx.send(()).unwrap();
        }

        // trace!("Sending audio frame");
        // if stream.send_audio_next_frame() {
        //     audio_tick_tx.send(()).unwrap();
        // };

        trace!("Sending video frame");
        stream.send_video_frame();

        std::thread::sleep(fps);
    }
}
