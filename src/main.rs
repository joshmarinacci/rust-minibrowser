#[macro_use]
extern crate glium;
extern crate glium_glyph;

use rust_minibrowser::dom::{Document, strip_empty_nodes, expand_entities};
use rust_minibrowser::layout;

use rust_minibrowser::style::{style_tree, expand_styles};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult, RenderInlineBoxType, EdgeSizes, Brush};
use rust_minibrowser::render::{FontCache};
use rust_minibrowser::net::{load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url, calculate_url_from_doc, BrowserError};
use url::Url;


use rust_minibrowser::app::{parse_args, navigate_to_doc, install_standard_fonts};

use cgmath::{Matrix4, Rad, Transform, Vector3, SquareMatrix};
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
                    dpi::PhysicalPosition,
                    event::ElementState,
};
use glium::{glutin, Display};
use glium::Surface;
use glium_glyph::GlyphBrush;
use glium_glyph::glyph_brush::{Section,
                               rusttype::{
                                   Font,
                                   Scale
                               }};
use rust_minibrowser::css::Color;
use rust_minibrowser::image::LoadedImage;
use std::collections::HashMap;
use glium::texture::{Texture2d, RawImage2d};
use std::rc::Rc;

const WIDTH:i32 = 800;
const HEIGHT:i32 = 800;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

implement_vertex!(Vertex, position, color);

#[derive(Copy, Clone)]
pub struct ImageVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],       // <- this is new
}
implement_vertex!(ImageVertex, position, tex_coords);        // don't forget to add `tex_coords` here

struct ImageRect {
    vertices:Vec<ImageVertex>,
    texture:Rc<Texture2d>,
}

pub fn make_box(shape:&mut Vec<Vertex>, rect:&Rect, color:&Color) {
    make_box2(shape, rect.x, rect.y, rect.x+rect.width, rect.y+rect.height, color);
}

pub fn make_box2(shape:&mut Vec<Vertex>, x1:f32,y1:f32,x2:f32,y2:f32, color:&Color) {
    shape.push(Vertex { position: [x1,  y1], color:color.to_array() });
    shape.push(Vertex { position: [ x2,  y1], color:color.to_array() });
    shape.push(Vertex { position: [ x2, y2], color:color.to_array() });

    shape.push(Vertex { position: [ x2, y2], color:color.to_array() });
    shape.push(Vertex { position: [x1, y2], color:color.to_array() });
    shape.push( Vertex { position: [x1,  y1], color:color.to_array() });
}

fn make_image_box(images:&mut Vec<ImageRect>, rect:&Rect, tex:&Rc<Texture2d>) {
    make_image_box2(images, rect.x, rect.y, rect.x+rect.width, rect.y+rect.height, tex);
}
fn make_image_box2(images:&mut Vec<ImageRect>, x1:f32, y1:f32, x2:f32, y2:f32, tex:&Rc<Texture2d>) {

    let vertex1 = ImageVertex { position: [x1, y1], tex_coords: [0.0, 0.0] };
    let vertex2 = ImageVertex { position: [x2, y1], tex_coords: [1.0, 0.0] };
    let vertex3 = ImageVertex { position: [x2, y2], tex_coords: [1.0, 1.0] };

    let vertex4 = ImageVertex { position: [x2, y2], tex_coords: [1.0, 1.0] };
    let vertex5 = ImageVertex { position: [x1, y2], tex_coords: [0.0, 1.0] };
    let vertex6 = ImageVertex { position: [x1, y1], tex_coords: [0.0, 0.0] };
    let ir = ImageRect {
        vertices:vec![vertex1, vertex2, vertex3, vertex4, vertex5, vertex6],
        texture:Rc::clone(tex),
    };
    images.push(ir)
}


pub fn make_border(shapes:&mut Vec<Vertex>, rect:&Rect, border_width:&EdgeSizes, color:&Color) {
    // println!("making border {:#?} {:#?}",border_width,color);
    //left
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y,
        width: border_width.left,
        height: rect.height
    }, color);
    //right
    make_box(shapes, &Rect {
        x: rect.x + rect.width - border_width.right,
        y: rect.y,
        width: border_width.right,
        height: rect.height
    }, color);

    //top
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: border_width.top
    }, color);
    //bottom
    make_box(shapes, &Rect {
        x: rect.x,
        y: rect.y+rect.height - border_width.bottom,
        width: rect.width,
        height: border_width.bottom
    }, color);
}

fn draw_render_box(bx:&RenderBox, gb:&mut FontCache, img:&mut HashMap<String, Rc<Texture2d>>, width:f32, height:f32, shapes:&mut Vec<Vertex>, images:&mut Vec<ImageRect>, text_scale:f32, display:&Display) {
    match bx {
        RenderBox::Block(rbx) => {
            // println!("box is {} border width {} {:#?}",rbx.title, rbx.border_width, rbx.padding);
            if let Some(color) = &rbx.background_color {
                make_box(shapes, &rbx.content_area_as_rect(), color);
            }
            if rbx.border_color.is_some() {
                let color = rbx.border_color.as_ref().unwrap();
                make_border(shapes, &rbx.content_area_as_rect(), &rbx.border_width, &color);
            }
            for ch in rbx.children.iter() {
                draw_render_box(ch, gb, img,width, height, shapes, images, text_scale, display);
            }
        }
        RenderBox::Anonymous(bx) => {
            for lb in bx.children.iter() {
                for inline in lb.children.iter() {
                    match inline {
                        RenderInlineBoxType::Text(text) => {
                            if text.color.is_some() && !text.text.is_empty() {
                                let color = text.color.as_ref().unwrap().clone();
                                let scale = Scale::uniform(text.font_size* text_scale);
                                let font = gb.lookup_font(&text.font_family, text.font_weight, &text.font_style);
                                let section = Section {
                                    text: &text.text.trim(),
                                    scale,
                                    font_id:*font,
                                    screen_position: (text.rect.x* text_scale, text.rect.y* text_scale),
                                    bounds: (text.rect.width* text_scale, text.rect.height* text_scale),
                                    color: [
                                        (color.r as f32)/255.0,
                                        (color.g as f32)/255.0,
                                        (color.b as f32)/255.0,
                                        (color.a as f32)/255.0,
                                    ],
                                    ..Section::default()
                                };
                                gb.brush.queue(section);
                                // make_box(shapes, &text.rect, &Color::from_hex("#ff0000"),scale_factor);
                            }
                        }
                        RenderInlineBoxType::Image(image) => {
                            if !img.contains_key(&*image.image.path) {
                                println!("must install the image");
                                let size = image.image.image2d.dimensions();
                                let data = image.image.image2d.clone().into_raw();
                                let tex_data:RawImage2d<u8> = RawImage2d::from_raw_rgba(data, size);
                                let texture = glium::texture::Texture2d::new(display, tex_data).unwrap();
                                img.insert(image.image.path.clone(),Rc::new(texture));
                            }
                            let tex_ref:&Rc<Texture2d> = img.get(image.image.path.as_str()).unwrap();
                            make_image_box(images, &image.rect, &tex_ref);
                            make_box(shapes, &image.rect, &Color::from_hex("#ff00ff"))
                        }
                        RenderInlineBoxType::Error(err) => {
                            make_box(shapes, &err.rect, &Color::from_hex("#ff00ff"))
                        }
                        RenderInlineBoxType::Block(block) => {
                            make_box(shapes, &block.rect, &Color::from_hex("#0000ff"))
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
    let mut font_cache =  FontCache {
        brush: Brush::Style1(GlyphBrush::new(&display, vec![])),
        families: Default::default(),
        fonts: Default::default()
    };
    install_standard_fonts(&mut font_cache);

    let start_page = parse_args().unwrap();
    let screen_dims = display.get_framebuffer_dimensions();
    let mut containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: screen_dims.0 as f32 / 2.0,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };
    let (mut doc, mut render_root) = navigate_to_doc(&start_page, &mut font_cache, containing_block).unwrap();


    let rect_vertex_shader_src = r#"
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

    let rect_fragment_shader_src = r#"
        #version 140

        out vec4 color;
        in vec4 f_color;

        void main() {
            color = f_color;
            //color = vec4(1.0, 1.0, 0.0, 1.0);
        }
    "#;

    let rect_program = glium::Program::from_source(&display, rect_vertex_shader_src, rect_fragment_shader_src, None).unwrap();

    let tex_vertex_shader_src = r#"
        #version 140

        in vec2 position;
        in vec2 tex_coords;
        out vec2 v_tex_coords;

        uniform mat4 matrix;

        void main() {
            v_tex_coords = tex_coords;
            gl_Position = matrix * vec4(position, 0.0, 1.0);
        }
    "#;
    let tex_fragment_shader_src = r#"
        #version 140

        in vec2 v_tex_coords;
        out vec4 color;

        uniform sampler2D tex;

        void main() {
            color = texture(tex, v_tex_coords);
        }
    "#;
    let tex_program = glium::Program::from_source(&display, tex_vertex_shader_src, tex_fragment_shader_src, None).unwrap();


    let mut yoff:f32 = 0.0;
    let zero:f32 = 0.0;
    let mut prev_w = screen_dims.0 as f32/2.0;
    let mut prev_h = screen_dims.1 as f32/2.0;
    let mut last_mouse:PhysicalPosition<f64> = PhysicalPosition{ x: 0.0, y: 0.0 };
    let mut image_cache:HashMap<String,Rc<Texture2d>> = HashMap::new();
    // main event loop
    event_loop.run(move |event, _tgt, control_flow| {
        *control_flow = ControlFlow::Wait;
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
                        LineDelta(x, y) => yoff = zero.max(yoff - y * 30.0),
                        PixelDelta(lp) => yoff = zero.max( yoff - lp.y as f32),
                    }
                },

                WindowEvent::CursorMoved {
                    device_id, position, modifiers
                } => {
                    last_mouse = position;
                }
                WindowEvent::MouseInput {
                    device_id, state, button, modifiers
                } => {
                    // println!("mouse click {:#?}", button);
                    if let ElementState::Pressed = state {
                        if let Left = button {
                            let res = render_root.find_box_containing((last_mouse.x / 2.0) as f32, (last_mouse.y / 2.0) as f32);
                            if let QueryResult::Text(bx) = res {
                                if let Some(href) = &bx.link {
                                    println!("following the link {:#?}", href);
                                    let url = calculate_url_from_doc(&doc, href).unwrap();
                                    let res = navigate_to_doc(&url, &mut font_cache, containing_block).unwrap();
                                    doc = res.0;
                                    render_root = res.1;
                                }
                            }
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
        let screen_dims = display.get_framebuffer_dimensions();
        let mut new_w = screen_dims.0 as f32/2.0;
        let mut new_h = screen_dims.1 as f32/2.0;
        if prev_w != new_w || prev_h != new_h {
            containing_block.content.width = new_w;
            let (mut doc2, mut render_root2) = navigate_to_doc(&start_page, &mut font_cache, containing_block).unwrap();
            doc = doc2;
            render_root = render_root2;
        }
        prev_w = new_w;
        prev_h = new_h;

        let mut shape:Vec<Vertex> = Vec::new();
        let mut images:Vec<ImageRect> = Vec::new();

        draw_render_box(&render_root, &mut font_cache, &mut image_cache,
                        new_w, new_h, &mut shape,  &mut images,2.0, &display);
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let (w,h) = display.get_framebuffer_dimensions();
        let w = w as f32;
        let h = h as f32;

        let box_translate = Matrix4::from_translation(Vector3{x: - 1.0, y:yoff/h + 1.0, z:0.0});
        let box_scale = Matrix4::from_nonuniform_scale(2.0*2.0/w,-2.0*2.0/h,1.0);
        let box_trans: [[f32; 4]; 4] = (box_translate * box_scale).into();
        let uniforms = uniform! { matrix: box_trans  };
        target.draw(&vertex_buffer, &indices, &rect_program, &uniforms, &Default::default()).unwrap();

        for image in images {
            let tex:&Texture2d = &image.texture;
            let image_uniforms = uniform! { matrix: box_trans, tex: tex };
            let img_vertex_buffer = glium::VertexBuffer::new(&display, &image.vertices).unwrap();
            target.draw(&img_vertex_buffer, &indices, &tex_program, &image_uniforms, &Default::default()).unwrap();
        }

        //draw fonts
        let scale = Matrix4::from_nonuniform_scale(2.0/w,  2.0/h, 1.0);
        let translate = Matrix4::from_translation(Vector3{ x: -1.0,  y: -1.0 - yoff/h,  z:0.0 });
        let transform: [[f32; 4]; 4] = (translate * scale).into();
        font_cache.brush.draw_queued_with_transform(transform, &display, &mut target);
        target.finish().unwrap();
    })
}
/*
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

*/

