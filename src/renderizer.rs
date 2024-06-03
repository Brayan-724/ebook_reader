use std::cell::RefCell;
use std::ops::DerefMut;
use std::sync::Arc;

use flo_render::OffscreenRenderContext;
use flo_render_canvas::render_canvas_offscreen;
use futures::lock::Mutex;
use futures::stream;

use crate::hot_lib;

pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 720;
pub const RESOLUTION: &str = "1280x720";

pub struct Renderizer<T> {
    render_context: RefCell<T>,
    context: RefCell<hot_lib::EbookContext>,
}

impl Renderizer<()> {
    pub fn new<T: OffscreenRenderContext>(render_context: T) -> Renderizer<T> {
        let context = RefCell::new(hot_lib::init());
        let render_context = RefCell::new(render_context);

        Renderizer {
            render_context,
            context,
        }
    }
}

impl<T: OffscreenRenderContext> Renderizer<T> {
    pub fn render(&mut self) -> Vec<u8> {
        futures::executor::block_on(self.render_async())
    }

    pub async fn render_async(&mut self) -> Vec<u8> {
        let context = self.context.get_mut();
        let drawing = hot_lib::render(vec![], context);

        // Render an image to bytes
        let render_context = self.render_context.get_mut();
        let image =
            render_canvas_offscreen(render_context, WIDTH, HEIGHT, 1.0, stream::iter(drawing))
                .await;

        image
    }
}
