use font_kit::font::Font;
use crate::dom::{BlockElem, Elem};
use crate::render::{Point, Size,BlockBox, RenderBox, LineBox,};
use crate::style::StyleManager;

pub fn perform_layout(dom:&BlockElem, styles:&StyleManager, font:&Font, width:i32) -> BlockBox {
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
                        let block_box = layout_div(font, &text.text, width);
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

fn layout_div(font:&Font, text:&str, width:i32) -> BlockBox {
    let _metrics = font.metrics();
    let mut block = BlockBox {
        pos: Point { x: 0, y: 0},
        size: Size { w: width, h: 10},
        boxes: Vec::new(),
    };
    let lines = layout_lines(font,text,width);
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

fn layout_lines(font:&Font, text:&str, width:i32)-> Vec<String>{
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
