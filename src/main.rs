mod render;
mod layout;
mod dom;

use dom::{BlockElem, Elem, TextElem};
use render::{drawBlockBox, drawRect, Point, Size};

use minifb::{ Window, WindowOptions,};
use raqote::{DrawTarget, SolidSource, PathBuilder, Source, DrawOptions};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use serde_json;
use serde_json::{Result, Value};
use std::fs;


const WIDTH: usize = 400;
const HEIGHT: usize = 400;


fn loadDoc(filename:&str) -> BlockElem {
    let data = fs::read_to_string(filename).expect("file shoudl open");
    let parsed:Value = serde_json::from_str(&data).unwrap();
    println!("parsed the type {}",parsed["type"]);

    let mut block = BlockElem {
        children: Vec::new(),
    };

    for child in parsed["children"].as_array().unwrap() {
        block.children.push(Elem::Text(TextElem {
            text:child["text"].as_str().unwrap().to_string()
        }))
    }

    let mut top = BlockElem {
        children: Vec::new(),
    };

    top.children.push(Elem::Block(block));

    return top;
}



fn main() {
    let mut window = Window::new("Raqote", WIDTH, HEIGHT, WindowOptions {
                                    ..WindowOptions::default()
                                }).unwrap();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    let size = window.get_size();
    
    let doc = loadDoc("test1.json");
    let bbox = layout::performLayout(&doc, &font, (size.0 - 100) as i32);
    let RED:Source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0x00, 0));

        let mut dt = DrawTarget::new(size.0 as i32, size.1 as i32);
    loop {
        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        drawBlockBox(&mut dt, &bbox, &font);

        // drawRect(&mut dt, &bbox.pos, &bbox.size, GREEN);
        // for lb in bbox.boxes.iter() {
        //     drawText(&mut dt, &font, &lb.pos, &lb.text);
        // }
        drawRect(&mut dt, &Point{x:(size.0 - 100) as i32, y:0}, &Size{w:1, h:200}, &RED);
        window.update_with_buffer(dt.get_data(), size.0, size.1).unwrap();
    }
}

