use rust_minibrowser::dom::{load_doc, getElementsByTagName, NodeType, Document};
use rust_minibrowser::style;
use rust_minibrowser::layout;

use minifb::{Window, WindowOptions, MouseButton, MouseMode};
use raqote::{DrawTarget, SolidSource, Source};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use rust_minibrowser::style::style_tree;
use rust_minibrowser::css::{load_stylesheet, parse_stylesheet, Stylesheet};
use rust_minibrowser::layout::{Dimensions, Rect, RenderBox, QueryResult};
use rust_minibrowser::render::draw_render_box;
use rust_minibrowser::net::load_doc_from_net;
use rust_minibrowser::globals::make_globals;
use font_kit::loaders::core_text::Font;
use reqwest::Url;
use std::string::ParseError;
use std::error::Error;
use std::env::current_dir;
use std::path::{PathBuf, Path};


const WIDTH: usize = 400;
const HEIGHT: usize = 600;

fn load_stylesheet_with_fallback(doc:&Document) -> Stylesheet {
    let style_node = getElementsByTagName(&doc.root_node, "style");
    let default_stylesheet = load_stylesheet("tests/default.css");

    match style_node {
        Some(node) => {
            if let NodeType::Text(text) = &node.children[0].node_type {
                let mut ss = parse_stylesheet(text);
                ss.parent = Some(Box::new(default_stylesheet));
                return ss
            }
        }
        _ => {}
    }
    return default_stylesheet;
}

fn navigate_to_doc(url:Url, font:&Font, containing_block:Dimensions) -> (Document, RenderBox) {
    let doc = load_doc_from_net(&url).unwrap();
    let stylesheet = load_stylesheet_with_fallback(&doc);
    let styled = style_tree(&doc.root_node,&stylesheet);
    let mut bbox = layout::build_layout_tree(&styled, &doc.base_url);
    let render_root = bbox.layout(containing_block, &font, &doc.base_url);
    return (doc,render_root)
}
fn relative_path_to_absolute_url(path:&str) -> Url {
    let cwd = current_dir().unwrap();
    let p = PathBuf::from(path);
    let final_path = cwd.join(p);;
    println!("final path is {}", final_path.display());
    let file_url_str = format!("file://{}",final_path.to_str().unwrap());
    println!("final path 2 is {}",file_url_str);
    let base_url = Url::parse(&*file_url_str).unwrap();
    println!("final url {}",base_url);
    return base_url;
}

fn main() {
    let globals = make_globals();
    let mut window = Window::new("Rust-Minibrowser", WIDTH, HEIGHT, WindowOptions {
        ..WindowOptions::default()
    }).unwrap();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

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
    let containing_block = Dimensions {
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

    let start_page = relative_path_to_absolute_url("tests/page1.html");
    // let start_page = Url::parse("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    let (mut doc, mut render_root) = navigate_to_doc(start_page, &font, containing_block);
    let mut dt = DrawTarget::new(size.width as i32, size.height as i32);
    let mut prev_left_down = false;
    loop {
        let left_down = window.get_mouse_down(MouseButton::Left);
        if left_down && !prev_left_down {
            let (x,y) = window.get_mouse_pos(MouseMode::Clamp).unwrap();
            println!("Left mouse is down at {} , {}",x,y);
            let res = render_root.find_box_containing(x,y);
            println!("got a result under the click: {:#?}", res);
            match res {
                QueryResult::Text(bx) => {
                    match &bx.link {
                        Some(href) => {
                            println!("going to load {} {}", href, doc.base_url);
                            let base_url = Url::parse(&*format!("file://{}", doc.base_url)).unwrap();
                            println!("base is {}", base_url);
                            let url = base_url.join(href).unwrap();
                            println!("going to the new url {}",url);
                            let res = navigate_to_doc(url, &font, containing_block);
                            doc = res.0;
                            render_root = res.1;
                        }
                        _ => {}
                    }
                }

                _ => {}
            }

        }
        prev_left_down = left_down;

        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        draw_render_box(&render_root, &mut dt, &font);
        window.update_with_buffer(dt.get_data(), size.width as usize, size.height as usize).unwrap();
    }
}

