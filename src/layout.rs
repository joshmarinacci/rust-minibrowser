use font_kit::font::Font;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::dom::{NodeType, load_doc};
// use crate::render::{Point, Size, BlockBox, LineBox, Inset};
use crate::render::Inset;
use crate::style::{StyledNode, style_tree};
use crate::css::load_stylesheet;
use crate::layout::BoxType::{BlockNode, InlineNode, AnonymousBlock};

#[derive(Debug, Default)]
pub struct Dimensions {
    content: Rect,
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

#[derive(Debug, Default)]
pub struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Default)]
pub struct EdgeSizes {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

#[derive(Debug)]
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

#[derive(Debug)]
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

#[derive(Debug)]
pub enum Display {
    Block,
    Inline,
    None,
}

fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Block => BlockNode(style_node),
        Inline => InlineNode(style_node),
        DisplayNone => panic!("Root node has display none.")
    });


    for child in &style_node.children {
        match child.display() {
            Block => {
                root.children.push(build_layout_tree(&child))
            },
            Inline => {
                root.get_inline_container().children.push(build_layout_tree(&child))
            },
            DisplayNone => {},
        }
    }
    return root;
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType<'a>) -> LayoutBox<'a> {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            BlockNode(_) => {
                // if last child is anonymous block, keep using it
                match self.children.last() {
                    Some(&LayoutBox { box_type: AnonymousBlock, ..}) => {},
                    _ => self.children.push(LayoutBox::new(AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}

/*
pub fn perform_layout<'a>(node:&'a StyledNode<'a>, font:&Font, width:f32) -> BlockBox<'a> {
    let bgc = node.color("background-color");
    let bdc = node.color("border-color");
    let mut bb = BlockBox {
        pos: Point { x: 0.0, y:0.0},
        size: Size { w: width, h: 10.0},
        boxes:Vec::<RenderBox>::new(),
        background_color:bgc,
        border_color:bdc,
        margin: Inset::empty(),
        border_width: Inset::empty(),
        padding: Inset::empty(),
    };
    let offset = Point{x:0.0,y:0.0};
    recurse_layout(&mut bb, node, font, width, &offset);
    return bb;
}
*/
/*
fn recurse_layout(root:&mut BlockBox, node:&StyledNode<'static>, font:&Font, width:f32, offset:&Point) -> f32 {
    match &node.node.node_type  {
        NodeType::Element(_block) => {
            let mut bb = BlockBox {
                pos: Point { x: offset.x, y:offset.y},
                size: Size { w: width, h: 10.0},
                boxes:Vec::<RenderBox>::new(),
                background_color:node.color("background-color"),
                border_color:node.color("border-color"),
                margin: Inset::same(node.insets("margin")),
                border_width: Inset::same(node.insets("border-width")),
                padding: Inset::same(node.insets("padding")),
            };
            let mut offset = Point {
                x: offset.x + bb.margin.left + bb.border_width.left + bb.padding.left,
                y: offset.y + bb.margin.top + bb.border_width.top +  bb.padding.top
            };
            let width = width - bb.margin.left - bb.border_width.left - bb.padding.left - bb.padding.right - bb.border_width.right - bb.margin.right;
            for elem in node.children.iter() {
                offset.y = recurse_layout(&mut bb, elem, font, width, &offset);
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
                    color: node.color("color"),
                }));

            }
            return offset.y;
        }
    }
}
*/

/*
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
*/
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

#[test]
fn test_layout<'a>() {
    let doc = load_doc("tests/test1.html");
    let stylesheet = load_stylesheet("tests/foo.css");
    let snode = style_tree(&doc,&stylesheet);
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let bnode = build_layout_tree(&snode);
    // let bnode = perform_layout(&snode, &font, 300 as f32);
    println!("final bnode is {:#?}",bnode)
}
