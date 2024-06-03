use flo_canvas::{Color, Draw, GraphicsContext, GraphicsPrimitives, Transform2D};

const WIDTH: f32 = 1280.;
const HEIGHT: f32 = 720.;

pub struct EbookContext {
    x: f32,
    y: f32,
}

#[no_mangle]
pub fn init() -> EbookContext {
    EbookContext { x: 0.0, y: 0.0 }
}

#[no_mangle]
pub fn render(mut drawing: Vec<Draw>, context: &mut EbookContext) -> Vec<Draw> {
    context.x += 2.0;
    if context.x >= WIDTH {
        context.x = 0.;
    }
    let x = context.x;

    context.y += 2.0;
    if context.y >= HEIGHT {
        context.y = 0.;
    }
    let y = context.y;

    // FIXME: Same error, canvas sizes are some weird. It wraps around some
    // unknown value.
    drawing.clear_canvas(Color::Rgba(0.9, 0.8, 0.8, 1.0));
    drawing.canvas_height(HEIGHT);
    drawing.transform(Transform2D::scale(1.0, 1.0));
    drawing.center_region(0., 0., WIDTH, HEIGHT);

    drawing.new_path();
    drawing.rect(-20., -20., 20., 20.);
    drawing.fill_color(Color::Rgba(0., 0., 1., 1.));
    drawing.fill();

    drawing.new_path();
    drawing.rect(x, y, x + 200., y + 200.);
    drawing.fill_color(Color::Rgba(1.0, 0.0, 0.0, 1.0));
    drawing.fill();

    drawing.new_path();
    drawing.rect(x - 20., y - 20., x + 20., y + 20.);
    drawing.fill_color(Color::Rgba(1.0, 0.0, 0.0, 1.0));
    drawing.fill();

    drawing
}
