use font_kit::font::Font;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::dom::{load_doc, NodeType};
use crate::style::{StyledNode, style_tree, Display};
use crate::css::{load_stylesheet, Color, Unit, Value};
use crate::layout::BoxType::{BlockNode, InlineNode, AnonymousBlock, InlineBlockNode};
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;
use crate::render::{BLACK};
use crate::image::{LoadedImage, load_image_from_path};
use std::path::Path;
use crate::net::{load_doc_from_net, load_image_from_net};

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
    InlineBlockNode(&'a StyledNode<'a>),
    AnonymousBlock(&'a StyledNode<'a>),
}

/*
block contains blocks
block contains anonymous which contains inlines

during layout phase
block_layout_children should loop through the children
if all block children, then do normal block routines
if all inline children, then do normal inline routines

*/

#[derive(Debug)]
pub enum RenderBox {
    Block(RenderBlockBox),
    Anonymous(RenderAnonymousBox),
    Inline(),
    InlineBlock(),
}
impl RenderBox {
    pub fn find_box_containing(&self, x:f32, y:f32) -> Option<RenderBox> {
        println!("checking at {} {}",x,y);
        return None;
    }
}

#[derive(Debug)]
pub struct RenderBlockBox {
    pub title: String,
    pub rect:Rect,
    pub margin:EdgeSizes,
    pub padding:EdgeSizes,
    pub background_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub children: Vec<RenderBox>,
}
#[derive(Debug)]
pub struct RenderAnonymousBox {
    pub(crate) rect:Rect,
    pub children: Vec<RenderLineBox>,
}
#[derive(Debug)]
pub struct RenderLineBox {
    pub(crate) rect:Rect,
    pub(crate) children: Vec<RenderInlineBoxType>,
}

#[derive(Debug)]
pub enum RenderInlineBoxType {
    Text(RenderTextBox),
    Image(RenderImageBox),
    Error(RenderErrorBox),
}

#[derive(Debug)]
pub struct RenderTextBox {
    pub(crate) rect:Rect,
    pub(crate) text:String,
    pub color:Option<Color>,
    pub font_size:f32,
}
#[derive(Debug)]
pub struct RenderImageBox {
    pub(crate) rect:Rect,
    pub(crate) image:LoadedImage,
}
#[derive(Debug)]
pub struct RenderErrorBox {
    pub(crate) rect:Rect,
}

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>, base_url:&str) -> LayoutBox<'a> {
    // println!("build_layout_tree {:#?}", style_node.node.node_type);
    // println!("styles {:#?}", style_node.specified_values);
    // println!("display is {:#?}", style_node.display());
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node),
        Display::Inline => InlineNode(style_node),
        Display::InlineBlock => InlineBlockNode(style_node),
        Display::None => panic!("Root node has display none.")
    });


    for child in &style_node.children {
        match child.display() {
            Display::Block => {
                // println!("block display for child {:#?}", child.node.node_type);
                root.children.push(build_layout_tree(&child, base_url))
            },
            Display::Inline => {
                // println!("inline display for child {:#?}", child.node.node_type);
                root.get_inline_container().children.push(build_layout_tree(&child, base_url))
            },
            Display::InlineBlock => {
                root.get_inline_container().children.push(build_layout_tree(&child, base_url))
            }
            Display::None => {
                // println!("skipping display for child {:#?}", child.node.node_type);
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
            BlockNode(node)
            | InlineNode(node)
            | InlineBlockNode(node)
            | AnonymousBlock(node) => node
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | InlineBlockNode(_) | AnonymousBlock(_) => self,
            BlockNode(node) => {
                // if last child is anonymous block, keep using it
                match self.children.last() {
                    Some(&LayoutBox { box_type: AnonymousBlock(node), ..}) => {},
                    _ => self.children.push(LayoutBox::new(AnonymousBlock(node))),
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    pub fn layout(&mut self, containing_block: Dimensions, font:&Font, base_url:&str) -> RenderBox {
        match self.box_type {
            BlockNode(_node) => {
                RenderBox::Block(self.layout_block(containing_block, font, base_url))
            },
            InlineNode(_node) => {
                RenderBox::Inline()
            },
            InlineBlockNode(_node) => {
                RenderBox::InlineBlock()
            }
            AnonymousBlock(_node) => {
                RenderBox::Anonymous(self.layout_anonymous(containing_block, font, base_url))
            },
        }
    }
    fn debug_calculate_element_name(&mut self) -> String{
        match self.box_type {
            BlockNode(sn) => match &sn.node.node_type {
                NodeType::Element(data) => data.tag_name.clone(),
                _ => "non-element".to_string(),
            }
            _ => "non-element".to_string(),
        }
    }
    fn layout_block(&mut self, containing_block: Dimensions, font:&Font, base_url:&str) -> RenderBlockBox {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        let children:Vec<RenderBox> = self.layout_block_children(font, base_url);
        self.calculate_block_height();
        return RenderBlockBox{
            rect:self.dimensions.content,
            margin: self.dimensions.margin,
            padding: self.dimensions.padding,
            children: children,
            title: self.debug_calculate_element_name(),
            background_color: self.get_style_node().color("background-color"),
            border_width: self.get_style_node().insets("border-width"),
            border_color: self.get_style_node().color("border-color"),
        }
    }
    fn layout_anonymous(&mut self, containing_block:Dimensions, font:&Font, base_url:&str) -> RenderAnonymousBox {
        let mut color = self.get_style_node().lookup_color("color", &BLACK);
        let font_size = self.get_style_node().lookup_length_px("font-size", 18.0);
        // println!("using the font size: {}",font_size);
        let d = &mut self.dimensions;
        let line_height = font_size*1.1;
        d.content.x = containing_block.content.x;
        d.content.width = containing_block.content.width;
        d.content.y = containing_block.content.height + containing_block.content.y;
        let mut lines:Vec<RenderLineBox> = vec![];
        // println!("doing anonymous block layout");
        let mut y = d.content.y;
        let mut len = 0.0;
        let mut line:String = String::new();
        let mut line_box = RenderLineBox {
            rect: Rect{
                x: d.content.x + 1.0,
                y: 0.0,
                width: d.content.width - 2.0,
                height: line_height - 2.0,
            },
            children: vec![]
        };
        let mut x = d.content.x;
        for child in &mut self.children {
            // println!("child node {:#?}",child.box_type);
            let mut color = color.clone();

            let is_inline_block = match child.box_type {
                InlineBlockNode(styled) => true,
                _ => false,
            };
            if is_inline_block {
                match layout_image(&child, x, y, line_height, base_url) {
                    Ok(blk) => {
                        x += blk.rect.width;
                        line_box.children.push(RenderInlineBoxType::Image(blk));
                    },
                    Err(blk) => {
                        x += blk.rect.width;
                        line_box.children.push(RenderInlineBoxType::Error(blk))
                    }
                }
            } else {
                let text = match child.box_type {
                    InlineNode(styled) => {
                        match &styled.node.node_type {
                            NodeType::Text(string) => string.clone(),
                            NodeType::Element(data) => {
                            // println!("got the styled node {:#?}",styled);
                                color = styled.lookup_color("color", &color);
                                if data.tag_name == "img" {
                                    "".to_string()
                                } else {
                                    match &styled.children[0].node.node_type {
                                        NodeType::Text(string) => string.clone(),
                                        _ => "".to_string()
                                    }
                                }
                            }
                            _ => {
                                "".to_string()
                            }
                        }
                    }
                    _ => "".to_string()
                };
                let text = text.trim();
                if text.len() <= 0 { continue; }

                let mut current_line = String::new();
                // println!("got the text {}", text);
                for word in text.split_whitespace() {
                    // println!("len is {}", len);
                    let wlen: f32 = calculate_word_length(word, font) / 2048.0 * 18.0;
                    if len + wlen > containing_block.content.width {
                        // println!("adding text for wrap -{}- {} : {}", current_line, x, len);
                        line_box.children.push(RenderInlineBoxType::Text(RenderTextBox {
                            rect: Rect {
                                x: x,
                                y: y + 2.0,
                                width: len,
                                height: line_height - 4.0,
                            },
                            text: current_line,
                            color: Some(color.clone()),
                            font_size,
                        }));

                        // println!("adding line box");
                        lines.push(line_box);
                        line_box = RenderLineBox {
                            rect: Rect {
                                x: d.content.x + 2.0,
                                y: 0.0,
                                width: 0.0,
                                height: 0.0
                            },
                            children: vec![]
                        };
                        len = 0.0;
                        line = String::new();
                        current_line = String::new();
                        d.content.height += line_height;
                        y += line_height;
                        x = d.content.x;
                    }
                    len += wlen;
                    line.push_str(word);
                    line.push_str(" ");
                    current_line.push_str(word);
                    current_line.push_str(" ");
                }
                // println!("ending text box -{}- at {} : {}",current_line,x,len);
                line_box.children.push(RenderInlineBoxType::Text(RenderTextBox {
                    rect: Rect {
                        x: x,
                        y: y + 2.0,
                        width: len,
                        height: line_height - 4.0,
                    },
                    text: current_line,
                    color: Some(color.clone()),
                    font_size,
                }));
                current_line = String::new();
                x += len;
            }
        }

        lines.push(line_box);
        d.content.height += line_height;
        y += line_height;

        return RenderAnonymousBox {
            rect: Rect {
                x: d.content.x+2.0,
                y: d.content.y+2.0,
                width: d.content.width-4.0,
                height: d.content.height-4.0,
            },
            children:lines,
        }
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
        //println!("final width is {} padding = {} margin: {}", d.content.width, d.padding.left, d.margin.left);
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

    fn layout_block_children(&mut self, font:&Font, base_url:&str) -> Vec<RenderBox>{
        let d = &mut self.dimensions;
        let mut children:Vec<RenderBox> = vec![];
        for child in self.children.iter_mut() {
            let bx = child.layout(*d,font, base_url);
            d.content.height = d.content.height + child.dimensions.margin_box().height;
            children.push(bx)
        };
        return children;
    }

    fn calculate_block_height(&mut self) {
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

}

fn layout_image(child:&LayoutBox, x:f32, y:f32, line_height:f32, base_url:&str) -> Result<RenderImageBox, RenderErrorBox> {
    let mut image_size = Rect { x:0.0, y:0.0, width: 30.0, height:30.0};
    let mut path = String::from("");
    match child.box_type {
        InlineBlockNode(styled) => {
            match &styled.node.node_type {
                NodeType::Element(data) => {
                    println!("looking at element data {:#?}", data);
                    let width = data.attributes.get("width").unwrap().parse::<u32>().unwrap();
                    println!("got width {}",width);
                    image_size.width = width as f32;

                    let height = data.attributes.get("height").unwrap().parse::<u32>().unwrap();
                    println!("got height {}",height);
                    image_size.height = height as f32;
                    let url = data.attributes.get("src").unwrap();
                    println!("got source {}",url);
                    // path = url;
                    println!("the base url is {}", base_url);
                    let page_path = Path::new(base_url);
                    let parent = page_path.parent().unwrap();
                    println!("parent path is {:#?}", parent);
                    let img_path = parent.join(url);
                    println!("image url is {:#?}",img_path);
                    path = String::from(img_path.to_str().unwrap());
                }
                _ => {}
            }
            // println!("checking the size {:#?}", styled);
        }
        _ => {}
    }
    let image = match path.starts_with("http") {
        true => {
            load_image_from_net(&path)
        },
        false => {
            load_image_from_path(&path)
        },
    };
    match image {
        Ok(image) => {
            Ok(RenderImageBox {
                rect: Rect {
                    x: x,
                    y: y - image_size.height + line_height,
                    width: image_size.width,
                    height: image_size.height,
                },
                image: image
            })
        },
        Err(str) => {
            println!("error loading the image for {} : {}", path, str);
            Err(RenderErrorBox {
                rect: Rect {
                    x: x,
                    y: y - image_size.height + line_height,
                    width: image_size.width,
                    height: image_size.height,
                },
            })
        }
    }
}

fn calculate_word_length(text:&str, font:&Font) -> f32 {
    let mut sum = 0.0;
    for ch in text.chars() {
        let gid = font.glyph_for_char(ch).unwrap();
        let len = font.advance(gid).unwrap().x;
        sum += len;
        // println!("   len of {} is {}",ch,len);
    }
    // println!("length of {} is {}",text, sum);
    return sum;
}

#[test]
fn test_layout<'a>() {
    let doc = load_doc_from_net("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    // let doc = load_doc("tests/image.html");
    let stylesheet = load_stylesheet("tests/default.css");
    // println!("stylesheet is {:#?}",stylesheet);
    let snode = style_tree(&doc.root_node,&stylesheet);
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    println!(" ======== build layout boxes ========");
    let mut root_box = build_layout_tree(&snode, &*doc.base_url);
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
    let render_box = root_box.layout(containing_block, &font, &doc.base_url);
    // println!("final render box is {:#?}", render_box);
    // dump_layout(&root_box,0);
}
fn expand_tab(tab:i32) -> String {
    let mut string = String::new();
    for _i in 0..tab {
        string.push(' ')
    }
    return string;
}
fn dump_layout(root:&LayoutBox, tab:i32) {
    let bt = match root.box_type {
        BlockNode(snode) => {
            let st = match &snode.node.node_type {
                NodeType::Text(_) => "text".to_string(),
                NodeType::Element(data) => format!("element \"{}\"",data.tag_name),
                NodeType::Meta(data) => format!("meta tag"),
            };
            format!("block {}",st)
        }
        InlineNode(snode) => {
            let st = match &snode.node.node_type {
                NodeType::Text(data) => format!("text \"{}\"",data),
                NodeType::Element(data) => format!("element {}",data.tag_name),
                NodeType::Meta(_data) => format!("meta tag"),
            };
            format!("inline {}",st)
        },
        InlineBlockNode(snode) => {
            format!("inline-block")
        }
        AnonymousBlock(_snode) => "anonymous".to_string(),
    };
    println!("{}layout {}", expand_tab(tab), bt);
    for child in root.children.iter() {
        dump_layout(child,tab+4);
    }
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
