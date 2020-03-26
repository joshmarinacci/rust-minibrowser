#[macro_use]
extern crate glium;
extern crate glium_glyph;

use rust_minibrowser::dom::{Document, strip_empty_nodes, expand_entities};
use rust_minibrowser::layout;

use rust_minibrowser::style::{style_tree, expand_styles};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult, RenderInlineBoxType, EdgeSizes};
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
                    event::MouseScrollDelta::{PixelDelta, LineDelta},
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
use rust_minibrowser::css::Color;

const WIDTH:i32 = 800;
const HEIGHT:i32 = 800;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

implement_vertex!(Vertex, position, color);

pub fn transform(x:f32, y:f32) -> (f32,f32){
    let w = WIDTH as f32;
    let h = HEIGHT as f32;
    return (x/w - 0.5 - 0.25 - 0.25, -y/h + 0.5 + 0.25 + 0.25);
}
pub fn make_box(shape:&mut Vec<Vertex>, rect:&Rect, color:&Color, sf:f32) {
    let (x1,y1) = transform(rect.x*sf,rect.y*sf);
    let (x2,y2) = transform((rect.x+rect.width)*sf,(rect.y+rect.height)*sf);
    make_box2(shape, x1, y1, x2, y2, color);
}

pub fn make_box2(shape:&mut Vec<Vertex>, x1:f32,y1:f32,x2:f32,y2:f32, color:&Color) {
    shape.push(Vertex { position: [x1,  y1], color:color.to_array() });
    shape.push(Vertex { position: [ x2,  y1], color:color.to_array() });
    shape.push(Vertex { position: [ x2, y2], color:color.to_array() });

    shape.push(Vertex { position: [ x2, y2], color:color.to_array() });
    shape.push(Vertex { position: [x1, y2], color:color.to_array() });
    shape.push( Vertex { position: [x1,  y1], color:color.to_array() });
}

pub fn make_border(shapes:&mut Vec<Vertex>, rect:&Rect, border_width:&EdgeSizes, color:&Color, sf:f32) {
    // println!("making border {:#?} {:#?}",border_width,color);
    //left
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y,
        width: border_width.left,
        height: rect.height
    }, color, sf);
    //right
    make_box(shapes, &Rect {
        x: rect.x + rect.width - border_width.right,
        y: rect.y,
        width: border_width.right,
        height: rect.height
    }, color, sf);

    //top
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: border_width.top
    }, color, sf);
    //bottom
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y+rect.height - border_width.bottom,
        width: rect.width,
        height: border_width.bottom
    }, color, sf);
}

pub fn draw_render_box(bx:&RenderBox, gb:&mut FontCache, width:f32, height:f32, scale_factor:f32, shapes:&mut Vec<Vertex>) {
    match bx {
        RenderBox::Block(rbx) => {
            // println!("box is {} border width {} {:#?}",rbx.title, rbx.border_width, rbx.padding);
            if let Some(color) = &rbx.background_color {
                make_box(shapes, &rbx.content_area_as_rect(), color, scale_factor);
            }
            if rbx.border_color.is_some() {
                let color = rbx.border_color.as_ref().unwrap();
                make_border(shapes, &rbx.content_area_as_rect(), &rbx.border_width, &color, scale_factor);
            }
            for ch in rbx.children.iter() {
                draw_render_box(ch, gb, width, height, scale_factor, shapes);
            }
        }
        RenderBox::Anonymous(bx) => {
            for lb in bx.children.iter() {
                for inline in lb.children.iter() {
                    match inline {
                        RenderInlineBoxType::Text(text) => {
                            if text.color.is_some() && !text.text.is_empty() {
                                let color = text.color.as_ref().unwrap().clone();
                                let scale = Scale::uniform(text.font_size * scale_factor as f32);
                                let section = Section {
                                    text: &*text.text,
                                    scale,
                                    screen_position: (text.rect.x*scale_factor, text.rect.y*scale_factor),
                                    bounds: (text.rect.width*scale_factor, text.rect.height*scale_factor),
                                    color: [
                                        (color.r as f32)/255.0,
                                        (color.g as f32)/255.0,
                                        (color.b as f32)/255.0,
                                        (color.a as f32)/255.0,
                                    ],
                                    ..Section::default()
                                };
                                gb.brush.queue(section);
                                // make_box(shape, &text.rect, &Color::from_hex("#ff0000"))
                                // draw_text(dt, font, &text.rect, &text.text, &color_to_source(&text.color.as_ref().unwrap()), text.font_size);
                            }
                        }
                        RenderInlineBoxType::Image(img) => {
                            make_box(shapes, &img.rect, &Color::from_hex("#00ff00"),scale_factor)
                        }
                        RenderInlineBoxType::Error(err) => {
                            make_box(shapes, &err.rect, &Color::from_hex("#ff00ff"),scale_factor)
                        }
                        RenderInlineBoxType::Block(block) => {
                            make_box(shapes, &block.rect, &Color::from_hex("#0000ff"),scale_factor)
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

    //load a font
    let open_sans_light: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Light.ttf");
    let open_sans_reg: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Regular.ttf");
    let open_sans_bold: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Bold.ttf");
    let fonts = vec![
        Font::from_bytes(open_sans_light).unwrap(),
        Font::from_bytes(open_sans_reg).unwrap(),
        Font::from_bytes(open_sans_bold).unwrap(),
    ];
    let mut font_cache =  FontCache {
        brush: GlyphBrush::new(&display, fonts),
            // .initial_cache_size((1024, 1024))
            // .build(factory.clone())
    };

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
        in vec4 color;
        out vec4 f_color;
        uniform mat4 matrix;

        void main() {
            f_color = color;
            gl_Position = matrix * vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        out vec4 color;
        in vec4 f_color;

        void main() {
            color = f_color;
            //color = vec4(1.0, 1.0, 0.0, 1.0);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut yoff:f32 = 0.0;
    let zero:f32 = 0.0;
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
                WindowEvent::MouseWheel {
                    delta,
                    ..
                } => {
                    match delta {
                        LineDelta(x, y) => yoff = zero.max(yoff - y * 10.0),
                        PixelDelta(lp) => yoff = zero.max( yoff - lp.y as f32),
                    }
                },
                _ => (),
            },
            _ => (),
        }
        let screen_dims = display.get_framebuffer_dimensions();
        let mut shape:Vec<Vertex> = Vec::new();

        draw_render_box(&render_root, &mut font_cache, screen_dims.0 as f32, screen_dims.1 as f32, 2.0, &mut shape);
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        //draw boxes
        let t = (yoff/800.0) as f32;
        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ 0.0 , t, 0.0, 1.0f32],
            ]
        };
        target.draw(&vertex_buffer, &indices, &program, &uniforms,
                    &Default::default()).unwrap();
        //draw fonts
        let dims = display.get_framebuffer_dimensions();
        let transform = [
            [2.0 / (dims.0 as f32), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (dims.1 as f32), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, -1.0 - t, 0.0, 1.0],
        ];
        font_cache.brush.draw_queued_with_transform(transform, &display, &mut target);
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

