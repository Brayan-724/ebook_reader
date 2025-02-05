pub mod config;
pub mod error;
mod logger;
mod render;
mod renderizer;
// mod streamer;
mod tts;
mod utils;

use std::io::Write;
use std::ops;
use std::sync::{Arc, Mutex};

use glib::object::ObjectExt;
use gst::message::StreamsSelected;
use gstreamer as gst;
use gstreamer_video as gst_video;

use gst::prelude::*;
use pango::prelude::*;

use error::EbookResult;
use log::{error, info};

fn main() {
    if let Err(err) = run() {
        if !log::log_enabled!(log::Level::Error) {
            env_logger::init();
        }

        error!("{err}");
    }
}

fn run() -> EbookResult<()> {
    env_logger::init();

    info!("PIPELINE CREATING");

    let pipeline = create_pipeline()?;

    info!("PIPELINE CREATED");

    pipeline.set_state(gst::State::Playing).unwrap();

    info!("PIPELINE PLAYING");

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::SECOND) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null).unwrap();

                let src = msg
                    .src()
                    .map(|s| s.path_string())
                    .unwrap_or_else(|| glib::GString::from("UNKNOWN"));
                let error = err.error();
                let debug = err.debug();
                error!("Received error from {src}: {error} (debug: {debug:?})");

                return Ok(()); // FIXME: Should be error
            }
            msg => info!("TICK {msg:#?}"),
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();

    Ok(())
}

fn create_pipeline() -> EbookResult<gst::Pipeline> {
    if let Err(err) = gst::init() {
        return Err(error::EbookError::Glib(err));
    };

    let stream = {
        let stream_key = std::env::var("TWITCH_STREAM_KEY").expect("No Stream Key");
        format!("rtmpsink location=rtmp://live.twitch.tv/app/{stream_key}")
    };

    let pipeline = format!("flvmux name=mux ! {stream} \
          audiotestsrc samplesperbuffer=44100 \
        ! mux. \
          videotestsrc is-live=true pattern=ball \
        ! video/x-raw,framerate=25/1 \
        ! x264enc \
        ! mux. \
    ");

    let mut context = gst::ParseContext::new();
    let pipeline =
        match gst::parse::launch_full(&pipeline, Some(&mut context), gst::ParseFlags::empty()) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                if let Some(gst::ParseError::NoSuchElement) = err.kind::<gst::ParseError>() {
                    error!("Missing element(s): {:?}", context.missing_elements());
                } else {
                    error!("Failed to parse pipeline: {err}");
                }

                std::process::exit(-1)
            }
        };

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::Pipeline");

    // let overlay = pipeline.by_name("overlay").expect("Sink element not found");

    // The PangoFontMap represents the set of fonts available for a particular rendering system.
    let fontmap = pangocairo::FontMap::new();
    // Create a new pango layouting context for the fontmap.
    let context = fontmap.create_context();
    // Create a pango layout object. This object is a string of text we want to layout.
    // It is wrapped in a LayoutWrapper (defined above) to be able to send it across threads.
    let layout = LayoutWrapper(pango::Layout::new(&context));

    // Select the text content and the font we want to use for the piece of text.
    let font_desc = pango::FontDescription::from_string("Sans Bold 26");
    layout.set_font_description(Some(&font_desc));
    layout.set_text("GStreamer");

    let drawer = Arc::new(Mutex::new(DrawingContext { layout, info: None }));

    // let (tick_tx, tick_rx) = mpsc::channel::<()>();
    // let (render_tx, render_rx) = mpsc::channel::<Vec<u8>>();
    // let render_rx = Arc::new(Mutex::new(render_rx));

    // thread::spawn(move || {
    //     let renderizer = initialize_offscreen_rendering().unwrap();
    //     let mut renderizer = Renderizer::new(renderizer);
    //
    //     loop {
    //         tick_rx.recv().unwrap();
    //
    //         let output = renderizer.render();
    //         render_tx.send(output).unwrap();
    //     }
    // });

    // overlay.connect_closure(
    //     "draw",
    //     false,
    //     glib::closure!(@strong drawer => move |_: &gst::Element, sample: &gst::Sample| {
    //         use std::f64::consts::PI;
    //
    //         let drawer = drawer.lock().unwrap();
    //
    //         let buffer = sample.buffer().unwrap();
    //         let timestamp = buffer.pts().unwrap();
    //
    //         let info = drawer.info.as_ref().unwrap();
    //         let layout = &drawer.layout;
    //
    //         let angle = 2.0 * PI * (timestamp % (10 * gst::ClockTime::SECOND)).nseconds() as f64
    //             / (10.0 * gst::ClockTime::SECOND.nseconds() as f64);
    //
    //         /* Create a Cairo image surface to draw into and the context around it. */
    //         let surface = cairo::ImageSurface::create(
    //             cairo::Format::ARgb32,
    //             info.width() as i32,
    //             info.height() as i32,
    //         )
    //         .unwrap();
    //         let cr = cairo::Context::new(&surface).expect("Failed to create cairo context");
    //
    //         cr.save().expect("Failed to save state");
    //         cr.set_operator(cairo::Operator::Clear);
    //         cr.paint().expect("Failed to clear background");
    //         cr.restore().expect("Failed to restore state");
    //
    //         // The image we draw (the text) will be static, but we will change the
    //         // transformation on the drawing context, which rotates and shifts everything
    //         // that we draw afterwards. Like this, we have no complicated calculations
    //         // in the actual drawing below.
    //         // Calling multiple transformation methods after each other will apply the
    //         // new transformation on top. If you repeat the cr.rotate(angle) line below
    //         // this a second time, everything in the canvas will rotate twice as fast.
    //         cr.translate(
    //             f64::from(info.width()) / 2.0,
    //             f64::from(info.height()) / 2.0,
    //         );
    //         cr.rotate(angle);
    //
    //         // This loop will render 10 times the string "GStreamer" in a circle
    //         for i in 0..10 {
    //             // Cairo, like most rendering frameworks, is using a stack for transformations
    //             // with this, we push our current transformation onto this stack - allowing us
    //             // to make temporary changes / render something / and then returning to the
    //             // previous transformations.
    //             cr.save().expect("Failed to save state");
    //
    //             let angle = (360. * f64::from(i)) / 10.0;
    //             let red = (1.0 + f64::cos((angle - 60.0) * PI / 180.0)) / 2.0;
    //             cr.set_source_rgb(red, 0.0, 1.0 - red);
    //             cr.rotate(angle * PI / 180.0);
    //
    //             // Update the text layout. This function is only updating pango's internal state.
    //             // So e.g. that after a 90 degree rotation it knows that what was previously going
    //             // to end up as a 200x100 rectangle would now be 100x200.
    //             pangocairo::functions::update_layout(&cr, layout);
    //             let (width, _height) = layout.size();
    //             // Using width and height of the text, we can properly position it within
    //             // our canvas.
    //             cr.move_to(
    //                 -(f64::from(width) / f64::from(pango::SCALE)) / 2.0,
    //                 -(f64::from(info.height())) / 2.0,
    //             );
    //             // After telling the layout object where to draw itself, we actually tell
    //             // it to draw itself into our cairo context.
    //             pangocairo::functions::show_layout(&cr, layout);
    //
    //             // Here we go one step up in our stack of transformations, removing any
    //             // changes we did to them since the last call to cr.save();
    //             cr.restore().expect("Failed to restore state");
    //         }
    //
    //         /* Drop the Cairo context to release the additional reference to the data and
    //          * then take ownership of the data. This only works if we have the one and only
    //          * reference to the image surface */
    //         drop(cr);
    //         let stride = surface.stride();
    //         let data = surface.take_data().unwrap();
    //
    //         /* Create an RGBA buffer, and add a video meta that the videooverlaycomposition expects */
    //         let mut buffer = gst::Buffer::from_mut_slice(data);
    //
    //         gst_video::VideoMeta::add_full(
    //             buffer.get_mut().unwrap(),
    //             gst_video::VideoFrameFlags::empty(),
    //             gst_video::VideoFormat::Bgra,
    //             info.width(),
    //             info.height(),
    //             &[0],
    //             &[stride],
    //         )
    //         .unwrap();
    //
    //         /* Turn the buffer into a VideoOverlayRectangle, then place
    //          * that into a VideoOverlayComposition and return it.
    //          *
    //          * A VideoOverlayComposition can take a Vec of such rectangles
    //          * spaced around the video frame, but we're just outputting 1
    //          * here */
    //         let rect = gst_video::VideoOverlayRectangle::new_raw(
    //             &buffer,
    //             0,
    //             0,
    //             info.width(),
    //             info.height(),
    //             gst_video::VideoOverlayFormatFlags::PREMULTIPLIED_ALPHA,
    //         );
    //
    //         gst_video::VideoOverlayComposition::new(Some(&rect))
    //             .unwrap()
    //         // tick_tx.send(()).unwrap();
    //         // let output = render_rx.lock().unwrap().recv().unwrap();
    //         // draw_overlay(output, sample)
    //     }),
    // );
    //
    // overlay.connect_closure(
    //     "caps-changed",
    //     false,
    //     glib::closure!(move |_overlay: &gst::Element,
    //                          caps: &gst::Caps,
    //                          _width: u32,
    //                          _height: u32| {
    //         let mut drawer = drawer.lock().unwrap();
    //         drawer.info = Some(gst_video::VideoInfo::from_caps(caps).unwrap());
    //     }),
    // );

    Ok(pipeline)
}

// fn _create_pipeline() {
//
//     let pipeline = gst::Pipeline::new();
//
//     let src = gst::ElementFactory::make("videotestsrc")
//         .property_from_str("pattern", "ball")
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//
//     let overlay = gst::ElementFactory::make("overlaycomposition")
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//
//     let caps = gst_video::VideoCapsBuilder::new()
//         .width(1280)
//         .height(720)
//         .framerate((15, 1).into())
//         .build();
//     let capsfilter = gst::ElementFactory::make("capsfilter")
//         .property("caps", &caps)
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//
//     let videoconvert = gst::ElementFactory::make("videoconvert")
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//     let enc = gst::ElementFactory::make("avenc_flv")
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//     let mux = gst::ElementFactory::make("flvmux")
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//     let sink = gst::ElementFactory::make("rtmpsink")
//         .property(
//             "location",
//             format!(
//                 "rtmp://live.twitch.tv/app/{}",
//                 std::env::var("TWITCH_STREAM_KEY").unwrap()
//             ),
//         )
//         .build()
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//
//     pipeline
//         .add_many([
//             &src,
//             &overlay,
//             &capsfilter,
//             &videoconvert,
//             &enc,
//             &mux,
//             &sink,
//         ])
//         .map_err(|err| error::EbookError::GlibBool(err))?;
//
//     gst::Element::link_many([
//         &src,
//         &overlay,
//         &capsfilter,
//         &videoconvert,
//         &enc,
//         &mux,
//         &sink,
//     ])
//     .map_err(|err| error::EbookError::GlibBool(err))?;
// }

// fn draw_overlay(
//     // renderizer: Arc<Mutex<Renderizer<impl OffscreenRenderContext>>>,
//     output: Vec<u8>,
//     sample: &gst::Sample,
// ) -> gst_video::VideoOverlayComposition {
//     // let mut renderizer = renderizer.lock().unwrap();
//     // let output = renderizer.render();
//     let buffer = sample.buffer().unwrap();
//     let info = buffer.pts();
//
//     /* Create a Cairo image surface to draw into and the context around it. */
//     let surface = cairo::ImageSurface::create(
//         cairo::Format::ARgb32,
//         info.width() as i32,
//         info.height() as i32,
//     )
//     .unwrap();
//     let cr = cairo::Context::new(&surface).expect("Failed to create cairo context");
//
//     /* Create an RGBA buffer, and add a video meta that the videooverlaycomposition expects */
//     let mut buffer = gst::Buffer::from_mut_slice(output);
//
//     gst_video::VideoMeta::add_full(
//         buffer.get_mut().unwrap(),
//         gst_video::VideoFrameFlags::empty(),
//         gst_video::VideoFormat::Bgra,
//         1280,
//         720,
//         &[0],
//         &[0],
//     )
//     .unwrap();
//
//     let rect = gst_video::VideoOverlayRectangle::new_raw(
//         &buffer,
//         0,
//         0,
//         1280,
//         720,
//         gst_video::VideoOverlayFormatFlags::PREMULTIPLIED_ALPHA,
//     );
//
//     gst_video::VideoOverlayComposition::new(Some(&rect)).unwrap()
// }

struct DrawingContext {
    layout: LayoutWrapper,
    info: Option<gst_video::VideoInfo>,
}

#[derive(Debug)]
struct LayoutWrapper(pango::Layout);

impl ops::Deref for LayoutWrapper {
    type Target = pango::Layout;

    fn deref(&self) -> &pango::Layout {
        assert_eq!(self.0.ref_count(), 1);
        &self.0
    }
}

// SAFETY: We ensure that there are never multiple references to the layout.
unsafe impl Send for LayoutWrapper {}

pub const PREVIEW_LOG: &str = "\x1b[1;36mPREVIEW\x1b[0m";
pub const VIDEO_LOG: &str = "\x1b[1;35mVIDEO\x1b[0m";
pub const AUDIO_LOG: &str = "\x1b[1;34mAUDIO\x1b[0m";
