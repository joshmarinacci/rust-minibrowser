use minifb::{ Window, WindowOptions,};
use raqote::{DrawTarget, SolidSource, PathBuilder, Source, DrawOptions};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
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
struct BlockBox {
    pos:Point,
    size:Size,
    boxes:Vec<LineBox>,
}
struct LineBox {
    pos:Point,
    text:String,
}

fn layoutTest(font:&Font, width:i32) -> BlockBox {
    let box1 = layoutDiv(font, "this is some long text that is so long it will have to wrap", width);
    return box1;
}

fn layoutDiv(font:&Font, text:&str, width:i32) -> BlockBox {
    let _metrics = font.metrics();
    let mut block = BlockBox {
        pos: Point { x: 0, y: 0},
        size: Size { w: width-100, h: 10},
        boxes: Vec::new(),
    };
    let lines = layoutLines(font,text,width);
    let mut y = 36;
    for line in lines {
        block.boxes.push(LineBox {
            pos: Point { x: 0, y: y},
            text: line.to_string(),
        });
        y += 36;
    };
    block.size.h = y;
    return block;
}

fn layoutLines(font:&Font, text:&str, width:i32)-> Vec<String>{
    let mut len = 0;
    let mut line:String = String::new();
    let mut lines:Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if len < 10 {
            println!("appending {} {}",word, len);
            len += word.len() as i32;
            line.push_str(word);
            line.push_str(" ");
        } else {
            lines.push(line);
            len = 0;
            line = String::new();
        }

    }
    
    lines.push(line);

    for line in lines.iter() {
        println!("line is {}",line);
    }
    return lines;
}

fn drawRect(dt: &mut DrawTarget, pos:&Point, size:&Size) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    dt.fill(&path, 
        &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0xff, 0)), 
        &DrawOptions::new());
}

fn drawText(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str) {
    dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)),
        &DrawOptions::new(),
   );    
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
    let bbox = layoutTest(&font, size.0 as i32);
    let mut dt = DrawTarget::new(size.0 as i32, size.1 as i32);
    loop {
        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        //drawRect(&mut dt, &bbox.pos, &bbox.size);
        for lb in bbox.boxes.iter() {
            drawText(&mut dt, &font, &lb.pos, &lb.text)
        }
        window.update_with_buffer(dt.get_data(), size.0, size.1).unwrap();
    }
}
