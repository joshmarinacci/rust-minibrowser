use font_kit::font::Font;
use crate::dom::{Elem};
use crate::render::{Point, Size,BlockBox, RenderBox, LineBox, Black, Green, Blue};
use crate::style::StyleManager;

pub fn perform_layout(dom:&Elem, styles:&StyleManager, font:&Font, width:i32) -> BlockBox {
    let mut bb = BlockBox {
        pos: Point { x: 0, y:0},
        size: Size { w: width, h: 10},
        boxes:Vec::<RenderBox>::new(),
        background_color:Green,
        border_color:Blue,
    };   
    let offset = Point{x:0,y:0};
    recurse_layout(&mut bb, dom, styles, font, width, &offset, 0);
    return bb;
}

fn recurse_layout(root:&mut BlockBox, dom:&Elem, styles:&StyleManager, font:&Font, width:i32, offset:&Point, yoff:i32) -> i32 {
    match dom  {
        Elem::Block(block) => {
            let mut bb = BlockBox {
                pos: Point { x: 0, y:yoff},
                size: Size { w: width, h: 10},
                boxes:Vec::<RenderBox>::new(),
                background_color:styles.find_background_color_for_type(&block.etype),
                border_color:styles.find_border_color(),
            };
            let mut offy = yoff;
            for elem in block.children.iter() {
                offy = recurse_layout(&mut bb, elem, styles, font, width, offset, offy);
            }
            bb.size.h = offy-bb.pos.y;
            root.boxes.push(RenderBox::Block(bb));
            return offy;
        },
        Elem::Text(text) => {
            let lines = layout_lines(font, &text.text, width);
            let mut offy = yoff;
            for line in lines.iter() {
                offy += 36;
                root.boxes.push(RenderBox::Line(LineBox{
                    pos: Point { x: 0, y: offy},
                    text: line.to_string(),
                    color:styles.find_color(),
                }));

            }
            return offy;
        }
    }
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
