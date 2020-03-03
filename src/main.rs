use rust_minibrowser::dom::{load_doc, getElementsByTagName, NodeType};
use rust_minibrowser::style;
use rust_minibrowser::layout;

use minifb::{ Window, WindowOptions,};
use raqote::{DrawTarget, SolidSource, Source};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use rust_minibrowser::style::style_tree;
use rust_minibrowser::css::{load_stylesheet, parse_stylesheet};
use rust_minibrowser::layout::{Dimensions, Rect};
use rust_minibrowser::render::draw_render_box;
use rust_minibrowser::net::load_doc_from_net;


const WIDTH: usize = 400;
const HEIGHT: usize = 600;

fn main() {


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

    let doc = load_doc_from_net("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    let style_node = getElementsByTagName(&doc.root_node, "style").unwrap();
    if let NodeType::Text(text) = &style_node.children[0].node_type {
        println!("got the text {}", text);
        let stylesheet = parse_stylesheet(text);
        println!("got the stylesheet {:#?}",stylesheet);
        // let stylesheet = load_stylesheet("tests/default.css");
        let styled = style_tree(&doc.root_node,&stylesheet);
        let mut bbox = layout::build_layout_tree(&styled);
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
        let render_root = bbox.layout(containing_block, &font);
        let red:Source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0x00, 0));


        let mut dt = DrawTarget::new(size.width as i32, size.height as i32);
        loop {
            dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
            draw_render_box(&render_root, &mut dt, &font);
            //fill_rect(&mut dt, &Point{x:size.w - 100.0, y:0.0}, &Size{w:1.0, h:size.h}, &red);
            window.update_with_buffer(dt.get_data(), size.width as usize, size.height as usize).unwrap();
        }
    }

}

