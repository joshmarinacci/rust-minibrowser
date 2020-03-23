use rust_minibrowser::dom::{Document, strip_empty_nodes, expand_entities};
use rust_minibrowser::layout;

use minifb::{Window, WindowOptions, MouseButton, MouseMode, KeyRepeat, Key};
use raqote::{DrawTarget, SolidSource, Transform};
use rust_minibrowser::style::{style_tree, expand_styles};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult};
use rust_minibrowser::render::{draw_render_box, FontCache};
use rust_minibrowser::net::{load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url, calculate_url_from_doc, BrowserError};
use url::Url;
use font_kit::source::SystemSource;
use font_kit::properties::Properties;
use std::env;
use rust_minibrowser::app::{parse_args, init_fonts, navigate_to_doc};


const WIDTH: usize = 800;
const HEIGHT: usize = 800;


fn main() -> Result<(),BrowserError>{
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
