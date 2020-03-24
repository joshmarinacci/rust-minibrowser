use rust_minibrowser::dom::{Document, strip_empty_nodes, expand_entities};
use rust_minibrowser::layout;

use rust_minibrowser::style::{style_tree, expand_styles};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult, RenderInlineBoxType};
use rust_minibrowser::render::{draw_render_box, FontCache};
use rust_minibrowser::net::{load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url, calculate_url_from_doc, BrowserError};
use url::Url;


use rust_minibrowser::app::{parse_args, init_fonts, navigate_to_doc};

use cgmath::{Matrix4, Rad, Transform, Vector3};
use gfx::{format::{Depth, Srgba8}, Device, Resources, Factory};
use gfx_glyph::*;
use glutin::{event::{
    ElementState, Event, KeyboardInput, ModifiersState, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
}, event_loop::ControlFlow, ContextWrapper};
use old_school_gfx_glutin_ext::*;
use std::{
    env,
    error::Error,
    f32::consts::PI as PI32,
    io::{self, Write},
};


const WIDTH: usize = 800;
const HEIGHT: usize = 800;

pub fn draw_boxes<A:Resources,B:Factory<A>>(bx:&RenderBox,
                                            gb:&mut GlyphBrush<A, B>,
                                            scale_factor:f32,
                                            width:f32,
                                            height:f32) {
    let text = "foo";
    // let width = 200.0;
    // let height = 100.0;
    match bx {
        RenderBox::Block(rbx) => {
            for ch in rbx.children.iter() {
                draw_boxes(ch, gb, scale_factor, width, height);
            }
        }
        RenderBox::Anonymous(bx) => {
            for lb in bx.children.iter() {
                // draw_boxes(ch, gb, scale, width, height);
                for inline in lb.children.iter() {
                    match inline {
                        RenderInlineBoxType::Text(text) => {
                            if text.color.is_some() && !text.text.is_empty() {
                                let color = text.color.as_ref().unwrap().clone();
                                let scale = Scale::uniform(text.font_size * scale_factor * 0.5);
                                let section = gfx_glyph::Section {
                                    text: &*text.text,
                                    scale,
                                    screen_position: (text.rect.x, text.rect.y),
                                    bounds: (width / 3.15, height),
                                    color: [
                                        (color.r as f32)/255.0,
                                        (color.g as f32)/255.0,
                                        0.3,
                                        1.0],
                                    ..Section::default()
                                };
                                gb.queue(section);
                                // draw_text(dt, font, &text.rect, &text.text, &color_to_source(&text.color.as_ref().unwrap()), text.font_size);
                            }
                        }
                        _ => {

                        }
                    }
                }
            }
        }
        _ => {}
    }
}


fn main() -> Result<(),BrowserError>{
    let start_page = parse_args().unwrap();
    println!("using the start page {}",start_page);

    //make an event loop
    let event_loop = glutin::event_loop::EventLoop::new();
    //build the window
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title("some title")
        .with_inner_size(glutin::dpi::PhysicalSize::new(1024, 576));
    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        glutin::ContextBuilder::new()
            .with_gfx_color_depth::<Srgba8, Depth>()
            .build_windowed(window_builder, &event_loop)?
            .init_gfx::<Srgba8, Depth>();

    //TODO: I don't know what this does
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    //TODO: loop helper. I don't really know what this does.
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut modifiers = ModifiersState::default();

    let mut font_size: f32 = 18.0;
    let text = "foo";

    //load a font
    let font: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Light.ttf");
    let mut glyph_brush =
        GlyphBrushBuilder::using_font_bytes(font)
        .initial_cache_size((1024, 1024))
        .build(factory.clone());

    let mut font_cache = init_fonts();
    let start_page = parse_args().unwrap();
    let mut containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: WIDTH as f32,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };
    let (mut doc, mut render_root) = navigate_to_doc(&start_page, &mut font_cache, containing_block).unwrap();
    let mut yoff = 0.0;
    // main event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            //TODO: just redraw on main events cleared. what does that mean?
            Event::MainEventsCleared => window_ctx.window().request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                //if esc or close requested, close the window
                WindowEvent::ModifiersChanged(new_mods) => modifiers = new_mods,
                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    window_ctx.resize(size);
                    window_ctx.update_gfx(&mut main_color, &mut main_depth);
                },
                WindowEvent::MouseWheel {
                    delta,
                    ..
                } => {
                    match delta {
                        MouseScrollDelta::LineDelta(x,y) => {
                            yoff += y;
                            yoff = yoff.min(0.0);
                        },
                        MouseScrollDelta::PixelDelta(lp) => {
                            yoff += lp.y as f32;
                            yoff = yoff.min(0.0);
                        },
                    }
                },
                _ => (),
            }
            Event::RedrawRequested(_) => {
                //TODO:  clear the window?
                encoder.clear(&main_color, [0.92, 0.92, 0.92, 1.0]);
                // i think this is the main color BUFFER
                let (width, height, ..) = main_color.get_dimensions();
                let (width, height) = (f32::from(width), f32::from(height));
                draw_boxes(&render_root, &mut glyph_brush, window_ctx.window().scale_factor() as f32, width, height);
                let projection: Matrix4<f32> = gfx_glyph::default_transform(&main_color).into();
                let offset = Matrix4::from_translation(Vector3::new(0.0, yoff, 0.0));
                glyph_brush
                    .use_queue()
                    .transform(projection * offset)
                    .draw(&mut encoder, &main_color)
                    .unwrap();
                encoder.flush(&mut device);
                window_ctx.swap_buffers().unwrap();
                device.cleanup();
                if let Some(rate) = loop_helper.report_rate() {
                    window_ctx
                        .window()
                        .set_title(&format!("{} - {:.0} FPS", "some text", rate));
                }

                loop_helper.loop_sleep();
                loop_helper.loop_start();
            },

            _ => (),
        }
    })
}
/*
fn main2() -> Result<(),BrowserError>{
    let mut window = Window::new("Rust-Minibrowser", WIDTH, HEIGHT, WindowOptions {
        title: true,
        resize: true,
        ..WindowOptions::default()
    }).unwrap();
    let mut font_cache = init_fonts();
    let start_page = parse_args().unwrap();

    let mut containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: WIDTH as f32,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };

    let (mut doc, mut render_root) = navigate_to_doc(&start_page, &mut font_cache, containing_block).unwrap();
    // let mut prev_left_down = false;
    // let mut prev_right_down = false;
    let mut prev_w = WIDTH;
    let mut prev_h = HEIGHT;
    let mut dt = DrawTarget::new(prev_w as i32, prev_h as i32);
    let mut viewport = Rect{
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    loop {
        let (w,h) = window.get_size();
        /*
        if w != prev_w || h != prev_h {
            println!("resized to {}x{}",w,h);
            dt = DrawTarget::new(w as i32, h as i32);
            viewport.width = w as f32;
            viewport.height = h as f32;
            containing_block.content.width = w as f32;
            let res = navigate_to_doc(&start_page, &mut font_cache, containing_block).unwrap();
            doc = res.0;
            render_root = res.1;
        }
        prev_w = w;
        prev_h = h;
        scroll_viewport(&window, &mut viewport);
        let ts = Transform::row_major(1.0, 0.0, 0.0, 1.0, viewport.x, -viewport.y);
        dt.set_transform(&ts);

        let right_down = window.get_mouse_down(MouseButton::Right);
        if right_down && !prev_right_down {
            let (x,y) = window.get_mouse_pos(MouseMode::Clamp).unwrap();
            println!("Left mouse is down at {} , {}",x,y);
            let res = render_root.find_box_containing(x,y);
            println!("got a result under the click: {:#?}", res);
        }
        let left_down = window.get_mouse_down(MouseButton::Left);
        if left_down && !prev_left_down {
            let (x,y) = window.get_mouse_pos(MouseMode::Clamp).unwrap();
            let res = render_root.find_box_containing(x,y);
            if let QueryResult::Text(bx) = res {
                if let Some(href) = &bx.link {
                    let res = navigate_to_doc(&calculate_url_from_doc(&doc,href).unwrap(), &mut font_cache, containing_block).unwrap();
                    doc = res.0;
                    render_root = res.1;
                }
            }

        }
        prev_left_down = left_down;
        */
        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        draw_render_box(&render_root, &mut dt, &mut font_cache, &viewport);
        window.update_with_buffer(dt.get_data(), w as usize, h as usize).unwrap();
    }
}
*/

