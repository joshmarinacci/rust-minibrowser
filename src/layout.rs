use font_kit::font::Font;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::dom::{load_doc};
use crate::style::{StyledNode, style_tree, Display};
use crate::css::load_stylesheet;
use crate::layout::BoxType::{BlockNode, InlineNode, AnonymousBlock};
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;

#[derive(Clone, Copy, Debug, Default)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
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
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
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

/*
block contains blocks
block contains anonymous which contains inlines

during layout phase
block_layout_children should loop through the children
if all block children, then do normal block routines
if all inline children, then do normal inline routines

*/

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    // println!("build_layout_tree {:#?}", style_node.node.node_type);
    // println!("styles {:#?}", style_node.specified_values);
    // println!("display is {:#?}", style_node.display());
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node),
        Display::Inline => InlineNode(style_node),
        Display::None => panic!("Root node has display none.")
    });


    for child in &style_node.children {
        match child.display() {
            Display::Block => {
                println!("block display for child {:#?}", child.node.node_type);
                root.children.push(build_layout_tree(&child))
            },
            Display::Inline => {
                println!("inline display for child {:#?}", child.node.node_type);
                root.get_inline_container().children.push(build_layout_tree(&child))
            },
            Display::None => {
                println!("skipping display for child {:#?}", child.node.node_type);
            },
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

    pub fn layout(&mut self, containing_block: Dimensions) {
        // println!("     layout {:#?}",self);
        match self.box_type {
            BlockNode(node) => {
                println!("doing block layout for {:#?}", node.node.node_type);
                // println!("doing layout for a block {:#?}", self.box_type);
                self.layout_block(containing_block)
            },
            InlineNode(node) => {
                println!("doing layout for an inline node {:#?}", node.node.node_type);
            },
            AnonymousBlock => {
                println!("doing layout for anonymous");
                self.layout_anonymous(containing_block)
            },
        }
    }
    fn layout_block(&mut self, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        self.layout_block_children();
        self.calculate_block_height();
        // println!("final dimensions for {:#?}", self.dimensions.content)
    }
    fn layout_anonymous(&mut self, containing_block:Dimensions) {
        self.layout_block_children();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    ///
    /// http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    ///
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    fn calculate_block_width(&mut self, containing_block:Dimensions) {
        let style = self.get_style_node();

        // 'width' has initial value 'auto'
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value of 0
        let zero = Length(0.0, Px);
        let mut margin_left = style.lookup("margin-left","margin", &zero);
        let mut margin_right = style.lookup("margin-right","margin", &zero);
        let border_left = style.lookup("border-left","border-width", &zero);
        let border_right = style.lookup("border-right","border-width", &zero);
        let padding_left = style.lookup("padding-left","padding", &zero);
        let padding_right = style.lookup("padding-right","padding", &zero);

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
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

        // Adjust used values so that the above sum equals `containing_block.width`.
        // Each arm of the `match` should increase the total width by exactly `underflow`,
        // and afterward all values should be absolute lengths in px.
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

        let d = &mut self.dimensions;
        d.content.width = width.to_px();
        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();
        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();
        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
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
    let doc = load_doc("tests/simple.html");
    let stylesheet = load_stylesheet("tests/default.css");
    // println!("stylesheet is {:#?}",stylesheet);
    let snode = style_tree(&doc,&stylesheet);
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    println!(" ======== build layout boxes ========");
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
    //println!("roob box is {:#?}",root_box);
    println!(" ======== layout phase ========");
    root_box.layout(containing_block);
    // println!("final bnode is {:#?}", root_box)
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
