use font_kit::font::Font;

use crate::dom::{NodeType, Document};
use crate::style::{StyledNode, Display, style_tree};
use crate::css::{Color, Unit, Value};
use crate::layout::BoxType::{BlockNode, InlineNode, AnonymousBlock, InlineBlockNode};
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;
use crate::render::{BLACK, FontCache};
use crate::image::{LoadedImage};
use crate::dom::NodeType::{Text, Element};
use crate::net::{load_image, load_stylesheet_from_net, relative_filepath_to_url, load_doc_from_net};
use crate::layout::RenderBox::Anonymous;
use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

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
    pub fn contains(self, x:f32, y:f32) -> bool {
        self.x <= x && self.x + self.width >= x && self.y <= y && self.y + self.height > y
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

#[derive(Debug)]
pub enum RenderBox {
    Block(RenderBlockBox),
    Anonymous(RenderAnonymousBox),
    Inline(),
    InlineBlock(),
}

#[derive(Debug)]
pub enum QueryResult<'a> {
    Text(&'a RenderTextBox),
    None(),
}
impl QueryResult<'_> {
    fn is_none(&self) -> bool {
        match self {
            QueryResult::None() =>true,
            _ => false
        }
    }
}


impl RenderBox {
    pub fn find_box_containing(&self, x:f32, y:f32) -> QueryResult {
        match self {
            RenderBox::Block(bx) => bx.find_box_containing(x,y),
            RenderBox::Anonymous(bx) => bx.find_box_containing(x,y),
            _ => QueryResult::None(),
        }
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

impl RenderBlockBox {
    pub fn find_box_containing(&self, x: f32, y: f32) -> QueryResult {
        for child in self.children.iter() {
            let res = child.find_box_containing(x,y);
            if !res.is_none() {
                return res
            }
        }
        QueryResult::None()
    }
    pub fn content_area_as_rect(&self) -> Rect {
        Rect {
            x: self.rect.x - self.padding.left - self.border_width,
            y: self.rect.y - self.padding.top - self.border_width,
            width: self.rect.width + self.padding.left + self.padding.right + self.border_width*2.0,
            height: self.rect.height + self.padding.top + self.padding.bottom + self.border_width*2.0
        }
    }
}

#[derive(Debug)]
pub struct RenderAnonymousBox {
    pub(crate) rect:Rect,
    pub children: Vec<RenderLineBox>,
}
impl RenderAnonymousBox {
    pub fn find_box_containing(&self, x: f32, y: f32) -> QueryResult {
        for child in self.children.iter() {
            let res = child.find_box_containing(x,y);
            if !res.is_none() {
                return res
            }
        }
        QueryResult::None()
    }
}
#[derive(Debug)]
pub struct RenderLineBox {
    pub(crate) rect:Rect,
    pub(crate) children: Vec<RenderInlineBoxType>,
}
impl RenderLineBox {
    pub fn find_box_containing(&self, x: f32, y: f32) -> QueryResult {
        for child in self.children.iter() {
            let res = match child {
                RenderInlineBoxType::Text(node) => node.find_box_containing(x,y),
                _ => QueryResult::None()
            };
            if !res.is_none() {
                return res
            }
        }
        QueryResult::None()
    }
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
    pub font_family:String,
    pub link:Option<String>,
    pub font_weight:f32,
}
impl RenderTextBox {
    pub fn find_box_containing(&self, x: f32, y: f32) -> QueryResult {
        if self.rect.contains(x,y) {
            return QueryResult::Text(&self)
        }
        QueryResult::None()
    }
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

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>, doc:&Document) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node),
        Display::Inline => InlineNode(style_node),
        Display::InlineBlock => InlineBlockNode(style_node),
        Display::None => panic!("Root node has display none.")
    });


    for child in &style_node.children {
        match child.display() {
            Display::Block =>  root.children.push(build_layout_tree(&child, doc)),
            Display::Inline => root.get_inline_container().children.push(build_layout_tree(&child, doc)),
            Display::InlineBlock => root.get_inline_container().children.push(build_layout_tree(&child, doc)),
            Display::None => {  },
        }
    }
    root
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType<'a>) -> LayoutBox<'a> {
        LayoutBox {
            box_type,
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
                    Some(&LayoutBox { box_type: AnonymousBlock(_node), ..}) => {},
                    _ => self.children.push(LayoutBox::new(AnonymousBlock(node))),
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    pub fn layout(&mut self, containing: &mut Dimensions, font:&mut FontCache, doc:&Document) -> RenderBox {
        match self.box_type {
            BlockNode(_node) =>         RenderBox::Block(self.layout_block(containing, font, doc)),
            InlineNode(_node) =>        RenderBox::Inline(),
            InlineBlockNode(_node) =>   RenderBox::InlineBlock(),
            AnonymousBlock(_node) =>    RenderBox::Anonymous(self.layout_anonymous_2(containing, font, doc)),
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
    fn layout_block(&mut self, containing_block: &mut Dimensions, font_cache:&mut FontCache, doc:&Document) -> RenderBlockBox {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        let children:Vec<RenderBox> = self.layout_block_children(font_cache, doc);
        self.calculate_block_height();
        RenderBlockBox{
            rect:self.dimensions.content,
            margin: self.dimensions.margin,
            padding: self.dimensions.padding,
            children,
            title: self.debug_calculate_element_name(),
            background_color: self.get_style_node().color("background-color"),
            border_width: self.get_style_node().insets("border-width"),
            border_color: self.get_style_node().color("border-color"),
        }
    }

    /*

    body
        text
        b
            second
            a
                third
            fourth
        fifth

    should become
        text
        b second
        b/a third
        b fourth
        fifth
    */
    /*
    fn make_flat_children(&self) -> Vec<LayoutBox>{
        // println!("making flat clhildren {:#?}", self);
        let mut v:Vec<LayoutBox> = vec![];

        for ch in &self.children {
            if let InlineNode(mut styled) = ch.box_type {
                match &styled.node.node_type {
                    Text(text) => {
                        println!("found a text inline {:#?}", text);
                        v.push(LayoutBox {
                            dimensions: Default::default(),
                            box_type: BoxType::InlineNode(styled),
                            children: vec![]
                        })
                    }
                    Element(ed) => {
                        // println!("found a nested child {:#?}",styled);
                        let mut v2 = ch.make_flat_children();
                        println!("element {}", ed.tag_name);
                        println!("self styles {:#?}", styled.specified_values);

                        for ch in v2.iter_mut() {
                            // ch.get_style_node().specified_values.insert(String::from("foo"), Value::Keyword(String::from("bar")));
                            // for (key,val) in &styled.specified_values {
                            //     let sn = ch.get_style_node();
                            //     sn.specified_values.insert(key.clone(),val.clone());
                            // }
                            println!("   {:#?} ",ch.get_style_node().node.node_type);
                        }
                        v.extend(v2);
                    }
                    _ => {}
                }
            }
            //println!("looking at child {:#?}",ch.box_type)
        }
        v
    }*/

    fn find_font_family(&mut self, font_cache:&mut FontCache) -> String {
        let font_family_values = self.get_style_node().lookup("font-family",
                                                              "font-family",
                                                              &Value::Keyword(String::from("sans-serif")));
        match font_family_values {
            Value::ArrayValue(vals ) => {
                for val in vals.iter() {
                    match val {
                        Value::StringLiteral(str) => {
                            if font_cache.has_font_family(str) {
                                return String::from(str);
                            }
                        }
                        Value::Keyword(str) => {
                            if font_cache.has_font_family(str) {
                                return String::from(str);
                            }
                        }
                        _ => {}
                    }
                }
                println!("no valid font found in stack: {:#?}",vals);
                String::from("sans-serif")
            }
            Value::Keyword(str) => str,
            _ => String::from("sans-serif"),
        }
    }

    fn get_type(&self) -> String {
        match self.box_type {
            BoxType::AnonymousBlock(styled)
            | BoxType::BlockNode(styled)
            | BoxType::InlineBlockNode(styled)
            | BoxType::InlineNode(styled) => format!("{:#?}",styled.node.node_type)
        }
    }

    //     do_inline_block_parent extents
    fn layout_anonymous_2(&mut self, dim:&mut Dimensions, font_cache:&mut FontCache, doc:&Document) -> RenderAnonymousBox {
        println!("parent is {:#?}",self.get_type());
        let line_height= 30.0;
        let mut looper = Looper {
            lines: vec![],
            current: RenderLineBox {
                rect: Rect{
                    x: dim.content.x,
                    y: dim.content.y + dim.content.height,
                    width: dim.content.width,
                    height: line_height,
                },
                children: vec![]
            },
            extents: Rect {
                x: dim.content.x,
                y: dim.content.height + dim.content.y,
                width: dim.content.width,
                height: line_height,
            },
            font_cache,
            doc,
        };
        for child in self.children.iter_mut() {
            println!("working on child {:#?}", child.get_type());
            match child.box_type {
                InlineBlockNode(_styled) => child.do_inline_block(&mut looper),
                InlineNode(_styled) => child.do_inline(&mut looper),
                _ => println!("cant do this child of an anonymous box"),
            }
        }
        println!("done with kids. current is {:#?}", looper.current);
        looper.lines.push(looper.current);
        looper.extents.x = 0.0;
        looper.extents.y += 20.0;
        dim.content.height += 20.0;
        println!("final lines is {:#?}", looper.lines);
        return RenderAnonymousBox {
            rect: Rect{
                x: dim.content.x,
                y: looper.extents.y,
                width: dim.content.width,
                height: looper.extents.height,
            },
            children: looper.lines,
        }
    }

    fn do_inline_block(&mut self, looper:&mut Looper) {
        let w = 100.0;
        if looper.extents.x + w > looper.extents.width {
            let old = mem::replace(&mut looper.current,RenderLineBox {
                rect: Default::default(),
                children: vec![]
            });
            looper.lines.push(old);
            looper.extents.x = 0.0;
            looper.extents.y += 20.0;
        }
        looper.current.children.push(RenderInlineBoxType::Error(RenderErrorBox {
            rect: Rect {
                x: looper.extents.x,
                y: looper.extents.y,
                width: w,
                height: 50.0,
            },
        }));
    }

    fn do_inline(&mut self, looper:&mut Looper) {
        if let BoxType::InlineNode(snode) = self.box_type {
            match &snode.node.node_type {
                NodeType::Text(txt) => {
                    let line_height = 30.0;
                    let font_weight = 400.0;
                    let font_size = 18.0;
                    let font_family = "sans-serif";
                    let mut curr_text = String::new();
                    let start_x = looper.extents.x;
                    for word in txt.trim().split_whitespace() {
                        println!("inline: working on text '{}'",word);
                        let font = looper.font_cache.get_font(&font_family, font_weight);
                        let w: f32 = calculate_word_length(word, font, font_size);
                        //let w = 30.0;
                        if looper.extents.x + w > looper.extents.width {
                            println!("too big, wrapping");
                            looper.current.children.push(RenderInlineBoxType::Text(RenderTextBox{
                                rect: Default::default(),
                                text: curr_text,
                                color: Some(BLACK),
                                font_size: font_size,
                                font_family: "sans-serif".to_string(),
                                link: None,
                                font_weight: font_weight,
                            }));
                            curr_text = String::new();
                            let old = mem::replace(&mut looper.current, RenderLineBox {
                                rect: Default::default(),
                                children: vec![],
                            });
                            looper.lines.push(old);
                            looper.extents.x = 0.0;
                            looper.extents.y += 20.0;
                        } else {
                            looper.extents.x += w;
                            curr_text.push_str(word);
                            curr_text.push_str(" ");
                            println!("appending '{}'",curr_text);
                        }
                    }
                    println!("adding what's left '{}'", curr_text);
                    //add in whatever is left
                    looper.current.children.push(RenderInlineBoxType::Text(RenderTextBox{
                        rect: Rect {
                            x: start_x,
                            y: looper.extents.y,
                            width: looper.extents.x,
                            height: line_height,
                        },
                        text: curr_text,
                        color: Some(BLACK),
                        font_size: font_size,
                        font_family: "sans-serif".to_string(),
                        link: None,
                        font_weight: font_weight,
                    }));

                }
                //     if child is element
                NodeType::Element(_ed) => {
                    for ch in self.children.iter_mut() {
                        ch.do_inline(looper);
                    }
                }
                _ => {}
            }
        }
    }


    fn layout_anonymous(&mut self, containing:Dimensions, font_cache:&mut FontCache, doc:&Document) -> RenderAnonymousBox {
        let color = self.get_style_node().lookup_color("color", &BLACK);
        let font_size = self.get_style_node().lookup_length_px("font-size", 18.0);
        let font_family = self.find_font_family(font_cache);
        let mut font_weight = self.get_style_node().lookup_font_weight(400.0);
        //println!("using the font: {}  size: {}  weight: {}",font_family, font_size, font_weight);
        let mut d = self.dimensions.clone();
        let line_height = font_size*1.1;
        d.content.x = containing.content.x;
        d.content.width = containing.content.width;
        d.content.y = containing.content.height + containing.content.y;
        let mut lines:Vec<RenderLineBox> = vec![];
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
        //let v2 = self.make_flat_children();
        let v2 = &self.children;
        println!("children are {:#?}",v2);
        for child in v2.iter() {
            /*
            to lay out an inline block we need to know the
                current line box
                current x extent
                max length of the line block
                the child to be laid out
                any inherited styles
            then it will
                make a render box or error box
                return the box
            after the block is laid out we need to
                add the inline box to the line box
                move the x extent and maybe y extent
                if the inline box was too long, then we need to finish the current line, start a new line, and add it there.

            to lay out a normal text block we need to know the
                current line box
                current x extent
                max length of the line block
                the child to be laid out
                any inherited styles
            then it will
                make the longest possible text box without wrapping
                make more line boxes with more text with recursing
                return the text boxes and line boxes
            after the inline is laid out we need to
                add the text boxes to the line box
                add any newly created line boxes
                move the x extent and y extent


            do_inline_block_parent extents
                make lines
                for child in children
                    if child is inline-block
                        do inline block(child, lines, current line box, extents, doc, fonts)
                        continue
                    if child is inline
                        do inline(child, lines, current line box, extents, doc, fonts)
                        continue
                add lines to a parent render box
                return

            do_block(child, lines, current line box, extents, doc, fonts)
                calculate internal block size
                load image
                create image-block-box or error-block-box
                if too wide
                    make new current line box
                    update extents
                add to current line box
                return

            do_inline(child, lines, current_line_box, extents, doc, fonts)
                if child is text
                    measure text to fit the max width
                        create text box
                        add to current line box
                    if wrap
                        add new line box to lines
                        measure more text
                        create text box
                        add to current line box
                if child is element
                    for ch in child
                        do_inline(ch, lines, current_line_box, extents)
                return
            */
            if let InlineBlockNode(_styled) = child.box_type {
                match layout_image(&child, x, y, line_height, doc) {
                    Ok(blk) => {
                        x += blk.rect.width;
                        line_box.children.push(RenderInlineBoxType::Image(blk));
                    },
                    Err(blk) => {
                        x += blk.rect.width;
                        line_box.children.push(RenderInlineBoxType::Error(blk))
                    }
                }
                continue;
            }
            let mut color = color.clone();
            let mut link:Option<&String> = Option::None;
            let text = match child.box_type {
                InlineNode(styled) => {
                    match &styled.node.node_type {
                        NodeType::Text(string) => string.clone(),
                        NodeType::Element(data) => {
                            // println!("got the styled node {:#?}",styled);
                            color = styled.lookup_color("color", &color);
                            font_weight = styled.lookup_font_weight(font_weight);
                            if data.tag_name == "a" {
                                link = data.attributes.get("href");
                            }
                            if data.tag_name == "img" {
                                "".to_string()
                            } else if styled.children.is_empty() {
                                // println!("WARNING: inline element without a text child {:#?}",child);
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
            if text.is_empty() { continue; }

            let mut current_line = String::new();
            // println!("got the text {}", text);
            for word in text.split_whitespace() {
                // println!("len is {}", len);
                let font = font_cache.get_font(&font_family, font_weight);
                let wlen: f32 = calculate_word_length(word, font, 10.0) / 2048.0 * 18.0;
                if len + wlen > containing.content.width {
                    // println!("adding text for wrap -{}- {} : {}", current_line, x, len);
                    line_box.children.push(RenderInlineBoxType::Text(RenderTextBox {
                        rect: Rect {
                            x,
                            y: y + 2.0,
                            width: len,
                            height: line_height - 4.0,
                        },
                        text: current_line,
                        color: Some(color.clone()),
                        font_size,
                        font_family:font_family.clone(),
                        font_weight,
                        link: link.map(String::from),
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
                    x,
                    y: y + 2.0,
                    width: len,
                    height: line_height - 4.0,
                },
                text: current_line,
                font_family:font_family.clone(),
                font_weight,
                color: Some(color.clone()),
                font_size,
                link: link.map(String::from),
            }));
            x += len;
            len = 0.0;
        }

        lines.push(line_box);
        d.content.height += line_height;
        self.dimensions = d;

        RenderAnonymousBox {
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
    fn calculate_block_width(&mut self, containing:&mut Dimensions) {
        let style = self.get_style_node();

        // 'width' has initial value 'auto'
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or_else(||auto.clone());

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
            &padding_left, &padding_right, &width].iter().map(|v| self.length_to_px(v)));
        if width != auto && total > containing.content.width {
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
        let underflow = containing.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false,false,false) => {
                margin_right = Length(self.length_to_px(&margin_right) + underflow, Px);
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
                    margin_right = Length(self.length_to_px(&margin_right) + underflow, Px);
                }
            }
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        self.dimensions.content.width = self.length_to_px(&width);
        self.dimensions.padding.left = self.length_to_px(&padding_left);
        self.dimensions.padding.right = self.length_to_px(&padding_right);
        self.dimensions.border.left = self.length_to_px(&border_left);
        self.dimensions.border.right = self.length_to_px(&border_right);
        self.dimensions.margin.left = self.length_to_px(&margin_left);
        self.dimensions.margin.right = self.length_to_px(&margin_right);
        //println!("final width is {} padding = {} margin: {}", d.content.width, d.padding.left, d.margin.left);
    }

    fn length_to_px(&self, value:&Value) -> f32{
        match value {
            Length(v, Unit::Px) => *v,
            Length(v, Unit::Em) => (*v)*30.0,
            _ => {0.0}
        }
    }
    fn calculate_block_position(&mut self, containing: &mut Dimensions) {
        let zero = Length(0.0, Px);
        let style = self.get_style_node();
        let margin = EdgeSizes {
            top: self.length_to_px(&style.lookup("margin-top", "margin", &zero)),
            bottom: self.length_to_px(&style.lookup("margin-bottom","margin",&zero)),
            ..(self.dimensions.margin)
        };
        let border = EdgeSizes {
            top: self.length_to_px(&style.lookup("border-top", "border-width", &zero)),
            bottom: self.length_to_px(&style.lookup("border-bottom","border-width",&zero)),
            ..(self.dimensions.border)
        };
        let padding = EdgeSizes {
            top: self.length_to_px(&style.lookup("padding-top", "padding", &zero)),
            bottom: self.length_to_px(&style.lookup("padding-bottom","padding",&zero)),
            ..(self.dimensions.padding)
        };

        self.dimensions.margin = margin;
        self.dimensions.border = border;
        self.dimensions.padding = padding;
        let d = &mut self.dimensions;
        d.content.x = containing.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing.content.height + containing.content.y + d.margin.top + d.border.top + d.padding.top;
    }

    fn layout_block_children(&mut self, font_cache:&mut FontCache, doc:&Document) -> Vec<RenderBox>{
        let d = &mut self.dimensions;
        let mut children:Vec<RenderBox> = vec![];
        for child in self.children.iter_mut() {
            let bx = child.layout(d, font_cache, doc);
            d.content.height += child.dimensions.margin_box().height;
            children.push(bx)
        };
        children
    }

    fn calculate_block_height(&mut self) {
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

}

fn layout_image(child:&LayoutBox, x:f32, y:f32, line_height:f32, doc:&Document) -> Result<RenderImageBox, RenderErrorBox> {
    let mut image_size = Rect { x:0.0, y:0.0, width: 30.0, height:30.0};
    let mut src = String::from("");
    if let InlineBlockNode(styled) = child.box_type {
        if let Element(data) = &styled.node.node_type {
            let width = if data.attributes.contains_key("width") {
                data.attributes.get("width").unwrap().parse::<u32>().unwrap()
            } else {
                100
            };
            image_size.width = width as f32;
            let height = if data.attributes.contains_key("height") {
                data.attributes.get("height").unwrap().parse::<u32>().unwrap()
            } else {
                100
            };
            image_size.height = height as f32;
            src = data.attributes.get("src").unwrap().clone();
        }
    }
    match load_image(&doc, &src) {
        Ok(image) => {
            Ok(RenderImageBox {
                rect: Rect {
                    x,
                    y: y - image_size.height + line_height,
                    width: image_size.width,
                    height: image_size.height,
                },
                image
            })
        },
        Err(err) => {
            println!("error loading the image for {} : {:#?}", src, err);
            Err(RenderErrorBox {
                rect: Rect {
                    x,
                    y: y - image_size.height + line_height,
                    width: image_size.width,
                    height: image_size.height,
                },
            })
        }
    }
}

fn calculate_word_length(text:&str, font:&Font, font_size:f32) -> f32 {
    let mut sum = 0.0;
    for ch in text.chars() {
        let gid = font.glyph_for_char(ch).unwrap();
        let len = font.advance(gid).unwrap().x / 2048.0 * font_size;
        sum += len;
    }
    println!("word len {} = {}", text, sum);
    sum
}

struct Looper<'a> {
    lines:Vec<RenderLineBox>,
    current: RenderLineBox,
    extents:Rect,
    font_cache:&'a mut FontCache,
    doc: &'a Document,
}


#[test]
fn test_layout<'a>() {
    let mut font_cache = FontCache::new();
    font_cache.install_font(&String::from("sans-serif"),
                            400.0,
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font(&String::from("sans-serif"),
                            700.0,
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());

    let doc = load_doc_from_net(&relative_filepath_to_url("tests/nested.html").unwrap()).unwrap();
    let ss_url = relative_filepath_to_url("tests/default.css").unwrap();
    let mut stylesheet = load_stylesheet_from_net(&ss_url).unwrap();
    font_cache.scan_for_fontface_rules(&stylesheet);
    let snode = style_tree(&doc.root_node,&stylesheet);
    println!(" ======== build layout boxes ========");
    let mut root_box = build_layout_tree(&snode, &doc);
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
    // println!("roob box is {:#?}",root_box);
    println!(" ======== layout phase ========");
    let render_box = root_box.layout(containing_block, &mut font_cache, &doc);
    // println!("final render box is {:#?}", render_box);
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}

