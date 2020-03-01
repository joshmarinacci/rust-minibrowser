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
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;

#[derive(Clone, Copy, Debug, Default)]
pub struct Dimensions {
    content: Rect,
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

impl Dimensions {
    fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
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

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) | InlineNode(node) => node,
            AnonymousBlock => panic!("anonymous block box has no style node")
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

    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) => {},
            AnonymousBlock => {},
        }
    }
    fn layout_block(&mut self, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        self.layout_block_children();
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block:Dimensions) {
        let style = self.get_style_node();
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());
        let zero = Length(0.0, Px);
        let mut margin_left = style.lookup("margin-left","margin", &zero);
        let mut margin_right = style.lookup("margin-right","margin", &zero);
        let border_left = style.lookup("border-left","border-width", &zero);
        let border_right = style.lookup("border-right","border-width", &zero);
        let padding_left = style.lookup("padding-left","padding", &zero);
        let padding_right = style.lookup("padding-right","padding", &zero);

        let total = sum([&margin_left, &margin_right, &border_left, &border_right,
            &padding_left, &padding_right, &width].iter().map(|v| v.to_px()));
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }
            if margin_right == auto {
                margin_right = Length(0.0,Px);
            }
        }

        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false,false,false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }
            (false,false,true) => { margin_right = Length(underflow, Px); }
            (false,true,false) => { margin_left = Length(underflow, Px); }
            (true, _, _) => {
                if margin_left == auto { margin_left = Length(0.0, Px); }
                if margin_right == auto { margin_right = Length(0.0, Px); }
                if underflow >= 0.0 {
                    width = Length(underflow, Px);
                } else {
                    width = Length(0.0, Px);
                    margin_right = Length(margin_right.to_px() + underflow, Px);
                }
            }
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        let zero = Length(0.0, Px);

        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom","margin",&zero).to_px();
        d.border.top = style.lookup("border-top", "border-width", &zero).to_px();
        d.border.bottom = style.lookup("border-bottom","border-width",&zero).to_px();
        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom","padding",&zero).to_px();
        d.content.x = containing_block.content.x +
            d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing_block.content.height + containing_block.content.y +
            d.margin.top + d.border.top + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(*d);
            d.content.height = d.content.height + child.dimensions.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
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
    let mut root_box = build_layout_tree(&snode);
    let containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };
    root_box.layout(containing_block);
    // let bnode = perform_layout(&snode, &font, 300 as f32);
    println!("final bnode is {:#?}", root_box)
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
