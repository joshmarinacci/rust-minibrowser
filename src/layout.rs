use font_kit::font::Font;
use crate::dom::{Elem, BlockElem};
use crate::render::{Point, Size,BlockBox, RenderBox, LineBox,};
use crate::style::StyleManager;

pub fn perform_layout(dom:&BlockElem, styles:&StyleManager, font:&Font, width:i32) -> BlockBox {
    let mut bb = BlockBox {
        pos: Point { x: 0, y:0},
        size: Size { w: width, h: 10},
        boxes:Vec::<RenderBox>::new(),
    };   
    let offset = Point{x:0,y:0};
    recurse_layout(&mut top, dom, font, width, &offset, 0);
    return RenderBox::Block(top);
}

fn recurse_layout(root:&mut BlockBox, dom:&Elem, font:&Font, width:i32, offset:&Point, yoff:i32) -> i32 {
    match dom  {
        Elem::Block(block) => {
            println!("has block child");
            let mut bb = BlockBox {
                pos: Point { x: 0, y:yoff},
                size: Size { w: width, h: 10},
                boxes:Vec::<RenderBox>::new(),
            };
            let mut offy = yoff;
            for elem in block.children.iter() {
                offy = recurse_layout(&mut bb, elem, font, width, offset, offy);
            }
            bb.size.h = offy-bb.pos.y;
            root.boxes.push(RenderBox::Block(bb));
            return offy;
        },
        Elem::Text(text) => {
            println!("has a text child");
            let lines = layoutLines(font, &text.text, width);
            let mut offy = yoff;
            for line in lines.iter() {
                offy += 36;
                root.boxes.push(RenderBox::Line(LineBox{
                    pos: Point { x: 0, y: offy},
                    text: line.to_string(),
                }));

            }
            return offy;
        }
    }
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
