use rust_minibrowser::dom::{Document};
use rust_minibrowser::layout;

use minifb::{Window, WindowOptions, MouseButton, MouseMode, KeyRepeat, Key};
use raqote::{DrawTarget, SolidSource, Transform};
use rust_minibrowser::style::style_tree;
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult};
use rust_minibrowser::render::{draw_render_box, FontCache};
use rust_minibrowser::net::{load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url, calculate_url_from_doc, BrowserError};
use url::Url;
use font_kit::source::SystemSource;
use font_kit::properties::Properties;
use std::env;


const WIDTH: usize = 800;
const HEIGHT: usize = 600;


fn navigate_to_doc(url:Url, font_cache:&mut FontCache, containing_block:Dimensions) -> Result<(Document, RenderBox),BrowserError> {
    let doc = load_doc_from_net(&url)?;
    let stylesheet = load_stylesheets_with_fallback(&doc)?;
    font_cache.scan_for_fontface_rules(&stylesheet);
    let styled = style_tree(&doc.root_node,&stylesheet);
    let mut bbox = layout::build_layout_tree(&styled, &doc);
    let render_root = bbox.layout(&mut containing_block.clone(), font_cache, &doc);
    Ok((doc,render_root))
}

fn init_fonts() -> FontCache {
    let mut font_cache = FontCache::new();
    font_cache.install_font(&String::from("sans-serif"),  400.0,&relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font(&String::from("sans-serif"),  700.0,&relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());
    font_cache.install_font_font(&String::from("monospace"),  400.0,SystemSource::new()
        .select_best_match(&[font_kit::family_name::FamilyName::Monospace], &Properties::new())
        .expect("monospace should be found")
        .load()
        .unwrap()
    );
    font_cache
}
fn main() -> Result<(),BrowserError>{
    let args: Vec<String> = env::args().collect();
    println!("args = {:?}", args);
    let mut window = Window::new("Rust-Minibrowser", WIDTH, HEIGHT, WindowOptions {
        ..WindowOptions::default()
    }).unwrap();
    let mut font_cache = init_fonts();
    let size = window.get_size();
    let size = Rect {
        x: 0.0,
        y: 0.0,
        width: size.0 as f32,
        height: size.1 as f32,
    };

    // let doc = load_doc_from_net("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();

    // let doc = load_doc("tests/simple.html");
    // let doc = load_doc("tests/image.html");
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
    // println!("render root is {:#?}",render_root);

    let mut start_page = relative_filepath_to_url("tests/page1.html")?;
    if args.len() > 1 {
        start_page = Url::parse(args[1].as_str())?;
    }

    // let start_page = relative_filepath_to_url("tests/nested.html")?;
    // let start_page = relative_filepath_to_url("tests/image.html")?;
    // let start_page = Url::parse("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    // let start_page = relative_filepath_to_url("tests/tufte/tufte.html")?;
    let (mut doc, mut render_root) = navigate_to_doc(start_page, &mut font_cache, containing_block).unwrap();
    let mut dt = DrawTarget::new(size.width as i32, size.height as i32);
    let mut prev_left_down = false;
    let mut viewport = Rect{
        x: 0.0,
        y: 0.0,
        width: WIDTH as f32,
        height: HEIGHT as f32,
    };
    loop {
        scroll_viewport(&window, &mut viewport);
        let ts = Transform::row_major(1.0, 0.0, 0.0, 1.0, viewport.x, -viewport.y);
        dt.set_transform(&ts);

        let left_down = window.get_mouse_down(MouseButton::Left);
        if left_down && !prev_left_down {
            let (x,y) = window.get_mouse_pos(MouseMode::Clamp).unwrap();
            println!("Left mouse is down at {} , {}",x,y);
            let res = render_root.find_box_containing(x,y);
            println!("got a result under the click: {:#?}", res);
            if let QueryResult::Text(bx) = res {
                if let Some(href) = &bx.link {
                    let res = navigate_to_doc(calculate_url_from_doc(&doc,href).unwrap(), &mut font_cache, containing_block).unwrap();
                    doc = res.0;
                    render_root = res.1;
                }
            }

        }
        prev_left_down = left_down;

        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        draw_render_box(&render_root, &mut dt, &mut font_cache, &viewport);
        window.update_with_buffer(dt.get_data(), size.width as usize, size.height as usize).unwrap();
    }
}

fn scroll_viewport(window:&Window, viewport:&mut Rect) {
    if let Some(keys) = window.get_keys_pressed(KeyRepeat::No) {
        for key in keys {
            match key {
                Key::Up    => viewport.y -= 100.0,
                Key::Down  => viewport.y += 100.0,
                Key::Left  => viewport.x += 100.0,
                Key::Right => viewport.x -= 100.0,
                _ => {}
            }
        }
    }
}
