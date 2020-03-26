#[macro_use]
extern crate glium;
extern crate glium_glyph;

use rust_minibrowser::dom::{Document, strip_empty_nodes, expand_entities};
use rust_minibrowser::layout;

use rust_minibrowser::style::{style_tree, expand_styles};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult, RenderInlineBoxType};
use rust_minibrowser::render::{FontCache};
use rust_minibrowser::net::{load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url, calculate_url_from_doc, BrowserError};
use url::Url;


use rust_minibrowser::app::{parse_args, navigate_to_doc};

use cgmath::{Matrix4, Rad, Transform, Vector3};
use glium::glutin::{Api,
                    GlProfile,
                    GlRequest,
                    window::WindowBuilder,
                    event_loop::ControlFlow,
                    event_loop::EventLoop,
                    event::WindowEvent,
                    event::StartCause,
                    event::VirtualKeyCode,
                    event::KeyboardInput,
                    event::Event,
                    ContextBuilder,
};
use glium::glutin;
use glium::Surface;
use glium_glyph::GlyphBrush;
use glium_glyph::glyph_brush::{Section,
                               rusttype::{
                                   Font,
                                   Scale
                               }};

const WIDTH:i32 = 800;
const HEIGHT:i32 = 800;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

pub fn transform(x:f32, y:f32) -> (f32,f32){
    let w = WIDTH as f32;
    let h = HEIGHT as f32;
    return (x/w - 0.5 - 0.25 - 0.25, -y/h + 0.5 + 0.25 + 0.25);
}
pub fn make_box(shape:&mut Vec<Vertex>, rect:&Rect) {
    let (x1,y1) = transform(rect.x,rect.y);
    let (x2,y2) = transform(rect.x+rect.width,rect.y+rect.height);
    make_box2(shape, x1, y1, x2, y2);
}

pub fn make_box2(shape:&mut Vec<Vertex>, x1:f32,y1:f32,x2:f32,y2:f32) {
    shape.push(Vertex { position: [x1,  y1] });
    shape.push(Vertex { position: [ x2,  y1] });
    shape.push(Vertex { position: [ x2, y2] });

    shape.push(Vertex { position: [ x2, y2] });
    shape.push(Vertex { position: [x1, y2] });
    shape.push( Vertex { position: [x1,  y1] });
}

pub fn draw_boxes(bx:&RenderBox, gb:&mut FontCache, width:f32, height:f32, scale_factor:f64, shape:&mut Vec<Vertex>) {
    match bx {
        RenderBox::Block(rbx) => {
            for ch in rbx.children.iter() {
                draw_boxes(ch, gb, width, height, scale_factor, shape);
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
                                let scale = Scale::uniform(text.font_size * scale_factor as f32);
                                let section = Section {
                                    text: &*text.text,
                                    scale,
                                    screen_position: (text.rect.x, text.rect.y),
                                    bounds: (text.rect.width*2.0, text.rect.height),
                                    color: [
                                        (color.r as f32)/255.0,
                                        (color.g as f32)/255.0,
                                        (color.b as f32)/255.0,
                                        (color.a as f32)/255.0,
                                    ],
                                    ..Section::default()
                                };
                                gb.brush.queue(section);
                                make_box(shape, &text.rect)
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
    let window = glutin::window::WindowBuilder::new()
        .with_title("some title")
        .with_inner_size(glutin::dpi::LogicalSize::new(WIDTH, HEIGHT));
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &event_loop).unwrap();

    //TODO: I don't know what this does
    // let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    //TODO: loop helper. I don't really know what this does.
    // let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    // let mut modifiers = ModifiersState::default();

    let mut font_size: f32 = 18.0;
    let text = "foo";

    //load a font
    let open_sans: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Light.ttf");
    let fonts = vec![Font::from_bytes(open_sans).unwrap()];
    let mut font_cache =  FontCache {
        // factory: factory.clone(),
        brush: GlyphBrush::new(&display, fonts),
            // .initial_cache_size((1024, 1024))
            // .build(factory.clone())
    };

    //let mut font_cache = init_fonts();
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
    let screen_dims = display.get_framebuffer_dimensions();


    let vertex_shader_src = r#"
        #version 140

        in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        out vec4 color;

        void main() {
            color = vec4(1.0, 1.0, 0.0, 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();


    // main event loop
    event_loop.run(move |event, _tgt, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
        let screen_dims = display.get_framebuffer_dimensions();
        let mut shape:Vec<Vertex> = Vec::new();

        draw_boxes(&render_root, &mut font_cache, screen_dims.0 as f32, screen_dims.1 as f32, 2.0, &mut shape);
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        //draw boxes
        target.draw(&vertex_buffer, &indices, &program, &glium::uniforms::EmptyUniforms,
                    &Default::default()).unwrap();
        //draw fonts
        font_cache.brush.draw_queued(&display, &mut target);
        target.finish().unwrap();
    })
}
/*

        let (w,h) = window.get_size();
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
    }
}
*/

