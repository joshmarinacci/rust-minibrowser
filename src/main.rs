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


struct Point {
    x:i32,
    y:i32,
}
struct Size {
    w:i32,
    h:i32,
}
enum RenderBox {
    Block(BlockBox),
    Line(LineBox),
}
struct BlockBox {
    pos:Point,
    size:Size,
    boxes:Vec<RenderBox>,
}
struct LineBox {
    pos:Point,
    text:String,
}

enum Elem {
    Block(BlockElem),
    Text(TextElem)
}
struct BlockElem {
    children: Vec<Elem>,
}

struct TextElem {
    text:String,
}

fn performLayout(dom:&BlockElem, font:&Font, width:i32) -> BlockBox {
    let mut bb = BlockBox {
        pos: Point { x: 0, y:0},
        size: Size { w: width, h: 10},
        boxes:Vec::<RenderBox>::new(),
    };
    for elem in dom.children.iter() {
        match elem {
            Elem::Block(block) => {
                println!("has block child");
                let first = &block.children[0];
                match first {
                    Elem::Block(block) => {
                        println!("blocks too deep!");
                    }
                    Elem::Text(text) => {
                        println!("has a text child");
                        let block_box = layoutDiv(font, &text.text, width);
                        println!("laid out the block box");
                        bb.boxes.push(RenderBox::Block(block_box));
                    }
                }
            },
            Elem::Text(text) => {
                println!("top elem has text child. it shouldn't!");
            }
        }
    }

    return bb;
}

fn layoutDiv(font:&Font, text:&str, width:i32) -> BlockBox {
    let _metrics = font.metrics();
    let mut block = BlockBox {
        pos: Point { x: 0, y: 0},
        size: Size { w: width, h: 10},
        boxes: Vec::new(),
    };
    let lines = layoutLines(font,text,width);
    let mut y = 36;
    for line in lines {
        block.boxes.push(RenderBox::Line(LineBox {
            pos: Point { x: 0, y: y},
            text: line.to_string(),
        }));
        y += 36;
    };
    block.size.h = y;
    return block;
}

fn layoutLines(font:&Font, text:&str, width:i32)-> Vec<String>{
    let mut len = 0.0;
    let mut line:String = String::new();
    let mut lines:Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let wlen:f32 = calculate_word_length(font, word)/60.0;
        if len + wlen > width as f32 {
            lines.push(line);
            len = 0.0;
            line = String::new();
        }
        len += wlen;
        line.push_str(word);
        line.push_str(" ");
    }
    
    lines.push(line);

    for line in lines.iter() {
        println!("line is {}",line);
    }
    return lines;
}

fn calculate_word_length(font:&Font, text:&str) -> f32 {
    let mut sum = 0.0;
    for ch in text.chars() {
        let gid = font.glyph_for_char(ch).unwrap();
        sum += font.advance(gid).unwrap().x;
    }
    return sum;
}

fn drawRect(dt: &mut DrawTarget, pos:&Point, size:&Size, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    dt.fill(&path, 
        color, 
        &DrawOptions::new());
}

fn drawText(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str) {
    dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)),
        &DrawOptions::new(),
   );    
}

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
    let bbox = performLayout(&doc, &font, (size.0 - 100) as i32);
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

fn drawBlockBox(dt:&mut DrawTarget, bb:&BlockBox, font:&Font) {
    let GREEN:Source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0xff, 0));
    drawRect(dt,&bb.pos, &bb.size, &GREEN);

    for child in bb.boxes.iter() {
        match child {
            RenderBox::Block(block) => {
                println!("rendering a block box");
                drawBlockBox(dt, &block, font);
            },
            RenderBox::Line(text) => {
                println!("rendering a text box");
                drawText(dt, &font, &text.pos, &text.text);
            }
        }
    }
}
