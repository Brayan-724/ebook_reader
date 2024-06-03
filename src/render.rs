use std::thread;

use flo_render::initialize_offscreen_rendering;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::{executor, SinkExt};
use log::{error, trace};

use crate::renderizer::Renderizer;
use crate::utils::get_last_message;
use crate::VIDEO_LOG;

pub struct EbookRenderer {
    frame_rx: UnboundedReceiver<Vec<u8>>,
    tick_tx: UnboundedSender<()>,
}

impl EbookRenderer {
    pub fn new() -> Self {
        let (frame_tx, frame_rx) = mpsc::unbounded::<Vec<u8>>();
        let (tick_tx, tick_rx) = mpsc::unbounded::<()>();

        Self::start_thread(frame_tx, tick_rx);

        Self { frame_rx, tick_tx }
    }

    pub fn send_tick(&mut self) {
        if let Err(err) = self.tick_tx.unbounded_send(()) {
            error!(target: VIDEO_LOG, "{err}");
        }
    }

    pub fn recv(&mut self) -> Option<Vec<u8>> {
        get_last_message(&mut self.frame_rx)
    }

    fn start_thread(frame_tx: UnboundedSender<Vec<u8>>, mut tick_rx: UnboundedReceiver<()>) {
        thread::spawn(move || {
            // Create an offscreen context
            let render_context = initialize_offscreen_rendering().unwrap();
            let mut renderizer = Renderizer::new(render_context);

            let mut tx = frame_tx;

            loop {
                let Some(_) = get_last_message(&mut tick_rx) else {
                    continue;
                };

                trace!(target: VIDEO_LOG, "RENDERING");

                executor::block_on(async {
                    let buf = renderizer.render_async().await;
                    if let Err(err) = tx.send(buf).await {
                        error!(target: VIDEO_LOG, "{err}");
                    }
                });
            }
        });
    }
}
