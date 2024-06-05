use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

use flo_render::OffscreenRenderContext;
use flo_render_canvas::render_canvas_offscreen;
use futures::stream;

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

pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 720;
pub const RESOLUTION: &str = "1280x720";

pub struct Renderizer<T> {
    render_context: Arc<Mutex<T>>,
    context: Arc<Mutex<hot_lib::EbookContext>>,
}

// unsafe impl<T> Send for Renderizer<T> {}

impl<T> Clone for Renderizer<T> {
    fn clone(&self) -> Self {
        Self {
            render_context: self.render_context.clone(),
            context: self.context.clone(),
        }
    }
}

impl Renderizer<()> {
    pub fn new<T: OffscreenRenderContext>(render_context: T) -> Renderizer<T> {
        let context = Arc::new(Mutex::new(hot_lib::init()));
        let render_context = Arc::new(Mutex::new(render_context));

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
        let mut context = self.context.lock().unwrap();
        let context = context.borrow_mut();
        let drawing = hot_lib::render(vec![], context);

        // Render an image to bytes
        let mut render_context = self.render_context.lock().unwrap();
        let render_context = render_context.borrow_mut() as &mut T;
        let image =
            render_canvas_offscreen(render_context, WIDTH, HEIGHT, 1.0, stream::iter(drawing))
                .await;

        image
    }
}
