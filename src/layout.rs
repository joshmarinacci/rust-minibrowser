use font_kit::font::Font;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::dom::{Node, NodeType, ElementData};
use crate::render::{Point, Size, BlockBox, RenderBox, LineBox, Inset, BLUE};
use crate::style::{StyleManager, ColorProps, InsetProps};

pub fn perform_layout(dom:&Node, styles:&StyleManager, font:&Font, width:f32) -> BlockBox {
    let mut bb = BlockBox {
        pos: Point { x: 0.0, y:0.0},
        size: Size { w: width, h: 10.0},
        boxes:Vec::<RenderBox>::new(),
        background_color:styles.find_color_prop_enum(ColorProps::background_color),
        border_color:styles.find_color_prop_enum(ColorProps::border_color),
        margin: Inset::empty(),
        border_width: Inset::empty(),
        padding: Inset::empty(),
    };
    let offset = Point{x:0.0,y:0.0};
    recurse_layout(&mut bb, dom, styles, font, width, &offset);
    return bb;
}

fn recurse_layout(root:&mut BlockBox, dom:&Node, styles:&StyleManager, font:&Font, width:f32, offset:&Point) -> f32 {
    match &dom.node_type  {
        NodeType::Element(block) => {
            let mut bb = BlockBox {
                pos: Point { x: offset.x, y:offset.y},
                size: Size { w: width, h: 10.0},
                boxes:Vec::<RenderBox>::new(),
                background_color:styles.find_color_prop_enum(ColorProps::background_color),
                border_color:styles.find_color_prop_enum(ColorProps::border_color),
                margin: styles.find_inset_prop_enum(InsetProps::margin),
                border_width: styles.find_inset_prop_enum(InsetProps::border_width),
                padding: styles.find_inset_prop_enum(InsetProps::padding),
            };
            let mut offset = Point {
                x: offset.x + bb.margin.left + bb.border_width.left + bb.padding.left,
                y: offset.y + bb.margin.top + bb.border_width.top +  bb.padding.top
            };
            let width = width - bb.margin.left - bb.border_width.left - bb.padding.left - bb.padding.right - bb.border_width.right - bb.margin.right;
            for elem in dom.children.iter() {
                offset.y = recurse_layout(&mut bb, elem, styles, font, width, &offset);
            }
            offset.y += bb.margin.top + bb.border_width.top + bb.padding.top + bb.padding.bottom +bb.border_width.top + bb.margin.top;
            bb.size.h = offset.y-bb.pos.y;
            root.boxes.push(RenderBox::Block(bb));
            return offset.y;
        },
        NodeType::Text(text) => {
            let lines = layout_lines(font, &text, width);
            let mut offset = Point { x: offset.x, y: offset.y};
            for line in lines.iter() {
                offset.y += 36.0;
                root.boxes.push(RenderBox::Line(LineBox{
                    pos: Point { x: offset.x, y: offset.y},
                    text: line.to_string(),
                    color:styles.find_color_prop_enum(ColorProps::color),
                }));

            }
            return offset.y;
        }
    }
}

#[test]
fn test_padding() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let mut sm = StyleManager::new();

    let mut div = Node {
        node_type: NodeType::Element(ElementData{
            tag_name: "div".to_string(),
            attributes: Default::default()
        }),
        children: vec![]
    };

    let rbox = perform_layout(&div, &sm, &font, 200.0);
    assert_eq!(rbox.size.w,200.0);
    assert_eq!(rbox.background_color,BLUE);
}

fn layout_lines(font:&Font, text:&str, width:f32)-> Vec<String>{
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
