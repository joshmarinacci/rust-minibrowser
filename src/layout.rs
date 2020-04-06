use crate::dom::{NodeType, Document, load_doc_from_bytestring};
use crate::style::{StyledNode, Display, dom_tree_to_stylednodes, expand_styles};
use crate::css::{Color, Unit, Value, parse_stylesheet_from_bytestring, Stylesheet};
use crate::layout::BoxType::{BlockNode, InlineNode, AnonymousBlock, InlineBlockNode, TableNode, TableRowGroupNode, TableRowNode, TableCellNode, ListItemNode};
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;
use crate::render::{BLACK, FontCache};
use crate::image::{LoadedImage};
use crate::dom::NodeType::{Text, Element};
use crate::net::{load_image, load_stylesheet_from_net, relative_filepath_to_url, load_doc_from_net, BrowserError};
use std::mem;
use glium_glyph::glyph_brush::{Section, rusttype::{Scale, Font}};
use glium_glyph::glyph_brush::GlyphCruncher;
use glium_glyph::glyph_brush::rusttype::Rect as GBRect;
use std::rc::Rc;

const FUDGE:f32 = 2.0;

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

#[derive(Clone, Copy, Debug)]
pub enum ListMarker {
    Disc,
    Decimal,
    None,
}

impl Rect {
    pub fn with_inset(self, val:f32) -> Rect {
        Rect {
            x: (self.x + val).floor() + 0.5,
            y: (self.y + val).floor() + 0.5,
            width: (self.width - val - val).floor(),
            height: (self.height - val -val).floor(),
        }
    }
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
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub children: Vec<LayoutBox>,
}

#[derive(Debug)]
pub enum BoxType {
    BlockNode(Rc<StyledNode>),
    InlineNode(Rc<StyledNode>),
    InlineBlockNode(Rc<StyledNode>),
    AnonymousBlock(Rc<StyledNode>),
    TableNode(Rc<StyledNode>),
    TableRowGroupNode(Rc<StyledNode>),
    TableRowNode(Rc<StyledNode>),
    TableCellNode(Rc<StyledNode>),
    ListItemNode(Rc<StyledNode>),
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
    pub border_width: EdgeSizes,
    pub valign:String,
    pub children: Vec<RenderBox>,
    pub marker:ListMarker,
    pub color:Option<Color>,
    pub font_size:f32,
    pub font_family:String,
    pub font_weight:i32,
    pub font_style:String,
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
            x: self.rect.x - self.padding.left - self.border_width.left,
            y: self.rect.y - self.padding.top - self.border_width.top,
            width: self.rect.width + self.padding.left + self.padding.right + self.border_width.left + self.border_width.right,
            height: self.rect.height + self.padding.top + self.padding.bottom + self.border_width.left + self.border_width.right,
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
    pub children: Vec<RenderInlineBoxType>,
    pub(crate) baseline:f32,
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
    Block(RenderBlockBox),
    Error(RenderErrorBox),
}

#[derive(Debug)]
pub struct RenderTextBox {
    pub rect:Rect,
    pub text:String,
    pub color:Option<Color>,
    pub font_size:f32,
    pub font_family:String,
    pub link:Option<String>,
    pub font_weight:i32,
    pub font_style:String,
    pub valign:String,
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
    pub rect:Rect,
    pub image:LoadedImage,
    pub valign:String,
}
#[derive(Debug)]
pub struct RenderErrorBox {
    pub rect:Rect,
    pub valign:String,
}

pub fn build_layout_tree<'a>(style_node: &Rc<StyledNode>, doc:&Document) -> LayoutBox {
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(Rc::clone(style_node)),
        Display::Inline => InlineNode(Rc::clone(style_node)),
        Display::InlineBlock => InlineBlockNode(Rc::clone(style_node)),
        Display::ListItem => BoxType::ListItemNode(Rc::clone(style_node)),
        Display::Table => TableNode(Rc::clone(style_node)),
        Display::TableRowGroup => TableRowGroupNode(Rc::clone(style_node)),
        Display::TableRow => TableRowNode(Rc::clone(style_node)),
        Display::TableCell => TableCellNode(Rc::clone(style_node)),
        Display::None => panic!("Root node has display none.")
    });


    for child in style_node.children.borrow().iter() {
        match child.display() {
            Display::Block =>  root.children.push(build_layout_tree(child, doc)),
            Display::ListItem =>  root.children.push(build_layout_tree(child, doc)),
            Display::Inline => root.get_inline_container().children.push(build_layout_tree(&child, doc)),
            Display::InlineBlock => root.get_inline_container().children.push(build_layout_tree(&child, doc)),
            Display::Table => root.children.push(build_layout_tree(&child,doc)),
            Display::TableRowGroup => root.children.push(build_layout_tree(&child, doc)),
            Display::TableRow => root.children.push(build_layout_tree(&child,doc)),
            Display::TableCell => root.children.push(build_layout_tree(&child,doc)),
            Display::None => {  },
        }
    }
    root
}

impl LayoutBox {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }
    fn get_style_node(&self) -> &Rc<StyledNode> {
        match &self.box_type {
            BlockNode(node)
            | TableNode(node)
            | TableRowGroupNode(node)
            | TableRowNode(node)
            | TableCellNode(node)
            | InlineNode(node)
            | InlineBlockNode(node)
            | ListItemNode(node)
            | AnonymousBlock(node) => &node
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox {
        match &self.box_type {
            InlineNode(_) | InlineBlockNode(_) | AnonymousBlock(_) | TableCellNode(_)=> self,
            BlockNode(node)
            | ListItemNode(node)
            | TableNode(node)
            | TableRowGroupNode(node)
            | TableRowNode(node) => {
                // if last child is anonymous block, keep using it
                let last = self.children.last();
                let is_anon = match last {
                    Some(ch) => {
                        match ch.box_type {
                            AnonymousBlock(_) => true,
                            _ => {false}
                        }
                    },
                    _ => {
                        false
                    }
                };
                if !is_anon {
                    // make new anon block
                    self.children.push(LayoutBox::new(AnonymousBlock(Rc::clone(node))))
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    pub fn layout(&mut self, containing: &mut Dimensions, font:&mut FontCache, doc:&Document) -> RenderBox {
        match &self.box_type {
            BlockNode(_node) =>         RenderBox::Block(self.layout_block(containing, font, doc)),
            TableNode(_node) =>         RenderBox::Block(self.layout_block(containing, font, doc)),
            TableRowGroupNode(_node) => RenderBox::Block(self.layout_block(containing, font, doc)),
            TableRowNode(_node) =>      RenderBox::Block(self.layout_table_row(containing, font, doc)),
            TableCellNode(_node) =>     RenderBox::Anonymous(self.layout_anonymous_2(containing, font, doc)),
            InlineNode(_node) =>        RenderBox::Inline(),
            InlineBlockNode(_node) =>   RenderBox::InlineBlock(),
            AnonymousBlock(_node) =>    RenderBox::Anonymous(self.layout_anonymous_2(containing, font, doc)),
            ListItemNode(_node) =>      RenderBox::Block(self.layout_block(containing, font, doc)),
        }
    }
    fn debug_calculate_element_name(&self) -> String{
        match &self.box_type {
            BlockNode(sn)
            | TableNode(sn)
            | TableRowGroupNode(sn)
            | TableRowNode(sn)
            | TableCellNode(sn)
            | InlineNode(sn)
            => match &sn.node.node_type {
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
        let zero = Length(0.0, Px);
        let style = self.get_style_node();
        // println!("border top for block is {} {:#?}", self.debug_calculate_element_name(), &style.lookup("border-top", "border-width", &zero));
        RenderBlockBox{
            rect:self.dimensions.content,
            margin: self.dimensions.margin,
            padding: self.dimensions.padding,
            children,
            title: self.debug_calculate_element_name(),
            background_color: style.color("background-color"),
            border_width: EdgeSizes {
                top: style.lookup_length_as_px("border-width-top", 0.0),
                bottom: style.lookup_length_as_px("border-width-bottom",0.0),
                left: style.lookup_length_as_px("border-width-top",0.0),
                right: style.lookup_length_as_px("border-width-bottom",0.0),
            },
            border_color: style.color("border-color"),
            valign: String::from("baseline"),
            marker: if style.lookup_string("display","block") == "list-item" {
                match &*style.lookup_string("list-style-type", "none") {
                    "disc" => ListMarker::Disc,
                    _ => ListMarker::None,
                }
            } else {
                ListMarker::None
            },
            color: Some(style.lookup_color("color", &BLACK)),
            font_family: style.lookup_font_family(font_cache),
            font_weight : style.lookup_font_weight(400),
            font_style : style.lookup_string("font-style", "normal"),
            font_size: style.lookup_font_size(),
        }
    }

    fn layout_table_row(&mut self, cb:&mut Dimensions, font_cache:&mut FontCache, doc: &Document) -> RenderBlockBox {
        // println!("layout_table_row");
        self.calculate_block_width(cb);
        self.calculate_block_position(cb);
        self.dimensions.content.height = 50.0;
        let mut children:Vec<RenderBox> = vec![];

        // println!("table row dims now {:#?}", self.dimensions);
        //count the number of table cell children
        let mut count = 0;
        for child in self.children.iter() {
            if let BoxType::TableCellNode(_) = child.box_type {
                count+= 1
            }
        }
        let child_width = self.dimensions.content.width / count as f32;
        for (index,child) in self.children.iter_mut().enumerate() {
            match child.box_type {
                BoxType::TableCellNode(_) => {
                    let mut cb = Dimensions {
                        content: Rect {
                            x: self.dimensions.content.x + child_width * (index as f32),
                            y: self.dimensions.content.y,
                            width: child_width,
                            height: 0.0
                        },
                        padding: Default::default(),
                        border: Default::default(),
                        margin: Default::default()
                    };
                    // println!("table cell child with count {} w = {} index = {} cb = {:#?}",count, child_width,index, cb);
                    let bx = child.layout(&mut cb, font_cache, doc);
                    // println!("table cell child created {:#?}",bx);
                    children.push(bx)
                }
                BoxType::AnonymousBlock(_)=>println!(" anonymous child"),
                _ => {
                    println!("table_row can't have child of {:#?}",child.get_type());
                }
            };
        };
        let zero = Length(0.0, Px);
        let style = self.get_style_node();
        RenderBlockBox {
            title: self.debug_calculate_element_name(),
            rect:self.dimensions.content,
            margin: self.dimensions.margin,
            padding: self.dimensions.padding,
            background_color: self.get_style_node().color("background-color"),
            border_width: EdgeSizes {
                top: style.lookup_length_as_px("border-width-top", 0.0),
                bottom: style.lookup_length_as_px("border-width-bottom",0.0),
                left: style.lookup_length_as_px("border-width-top",0.0),
                right: style.lookup_length_as_px("border-width-bottom",0.0),
            },
            border_color: self.get_style_node().color("border-color"),
            valign: String::from("baseline"),
            children: children,
            marker: ListMarker::None,
            color: Some(style.lookup_color("color", &BLACK)),
            font_family: style.lookup_font_family(font_cache),
            font_weight : style.lookup_font_weight(400),
            font_style : style.lookup_string("font-style", "normal"),
            font_size: style.lookup_font_size(),
        }
    }

    fn get_type(&self) -> String {
        match &self.box_type {
            BoxType::AnonymousBlock(styled)
            | BoxType::ListItemNode(styled)
            | BoxType::BlockNode(styled)
            | BoxType::TableNode(styled)
            | BoxType::TableRowGroupNode(styled)
            | BoxType::TableRowNode(styled)
            | BoxType::TableCellNode(styled)
            | BoxType::InlineBlockNode(styled)
            | BoxType::InlineNode(styled) => format!("{:#?}",styled.node.node_type)
        }
    }

    fn layout_anonymous_2(&mut self, dim:&mut Dimensions, font_cache:&mut FontCache, doc:&Document) -> RenderAnonymousBox {
        // println!("parent is {:#?}",self.get_type());
        // println!("parent style node is {:#?}",self.get_style_node());
        let mut looper = Looper {
            lines: vec![],
            current: RenderLineBox {
                rect: Rect{
                    x: dim.content.x,
                    y: dim.content.y + dim.content.height,
                    width: dim.content.width,
                    height: 0.0,
                },
                baseline:0.0,
                children: vec![]
            },
            extents: Rect {
                x: dim.content.x,
                y: dim.content.y + dim.content.height,
                width: dim.content.width,
                height: 0.0,
            },
            current_start: dim.content.x,
            current_end: dim.content.x,
            current_bottom: dim.content.y + dim.content.height,
            font_cache:font_cache,
            doc,
            style_node:Rc::clone(self.get_style_node()),
        };
        for child in self.children.iter_mut() {
            // println!("working on child {:#?}", child.get_type());
            // println!("current start and end is {} {} ",looper.current_start, looper.current_end);
            match &child.box_type {
                InlineBlockNode(_styled) => child.do_inline_block(&mut looper),
                InlineNode(_styled) => child.do_inline(&mut looper),
                _ => println!("cant do this child of an anonymous box"),
            }
            // println!("and now after it is {} {}", looper.current_start, looper.current_end)
        }
        looper.adjust_current_line_vertical();
        let old = looper.current;
        looper.current_bottom += old.rect.height;
        looper.extents.height += old.rect.height;
        looper.lines.push(old);
        self.dimensions.content.y = looper.extents.y;
        self.dimensions.content.width = looper.extents.width;
        self.dimensions.content.height = looper.current_bottom - looper.extents.y ;
        // println!("at the end of the looper, bottom = {} y = {} h = {}",
        //          looper.current_bottom, self.dimensions.content.y, self.dimensions.content.height);
        // println!("line boxes are");
        // for line in looper.lines.iter() {
        //     println!("  line {:#?}",line.rect);
        // }
        RenderAnonymousBox {
            rect: looper.extents,
            children: looper.lines,
        }
    }

    fn do_inline_block(&mut self, looper:&mut Looper) {
        let mut image_size = Rect { x:0.0, y:0.0, width: 30.0, height:30.0};
        let mut src = String::from("");
        // let w = 100.0;
        if let InlineBlockNode(styled) = &self.box_type {
            if let Element(data) = &styled.node.node_type {
                match data.tag_name.as_str() {
                    "img" => {
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
                    },
                    "button" => {
                        // let font_family = self.find_font_family(looper.font_cache);
                        let font_family = "sans-serif";
                        let font_weight = self.get_style_node().lookup_font_weight(400);
                        let font_size = self.get_style_node().lookup_font_size();
                        let font_style = self.get_style_node().lookup_string("font-style", "normal");
                        println!("button font size is {}",font_size);
                        // let font = looper.font_cache.get_font(&font_family, font_weight, &font_style);
                        let text_node = &styled.children.borrow()[0].node;
                        let text = match &text_node.node_type {
                            NodeType::Text(str) => str,
                            _ => panic!("can't do inline block layout if child isn't text"),
                        };
                        let w: f32 = calculate_word_length(&text, looper.font_cache, font_size, &font_family, font_weight, &font_style);
                        println!("calculated width is {}",w);
                        looper.current_end += w;
                        let mut containing_block = Dimensions {
                            content: Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 50.0,
                                height: 0.0,
                            },
                            padding: Default::default(),
                            border: Default::default(),
                            margin: Default::default()
                        };
                        // let mut block = self.layout_block(&mut containing_block, looper.font_cache, looper.doc);
                        // block.rect.x = looper.current_start;
                        // block.rect.y = looper.current.rect.y;
                        // block.valign = self.get_style_node().lookup_string("vertical-align","baseline");
                        // let rbx = RenderInlineBoxType::Block(block);
                        // looper.add_box_to_current_line(rbx);
                        return;
                    },
                    _ => {
                        panic!("We don't handle inline-block on non-images yet: tag_name={}",data.tag_name);
                    },
                }
            }
        }

        let bx = match load_image(looper.doc, &src) {
            Ok(image) => {
                println!("Loaded the image {} {}", image.width, image.height);
                RenderInlineBoxType::Image(RenderImageBox {
                    rect: Rect {
                        x:looper.current_start,
                        y: looper.current.rect.y,
                        width: image.width as f32,
                        height: image.height as f32,
                    },
                    valign: self.get_style_node().lookup_string("vertical-align","baseline"),
                    image
                })
            },
            Err(err) => {
                println!("error loading the image for {} : {:#?}", src, err);
                RenderInlineBoxType::Error(RenderErrorBox {
                    rect: Rect {
                        x:looper.current_start,
                        y: looper.current.rect.y,
                        width: image_size.width,
                        height: image_size.height,
                    },
                    valign: self.get_style_node().lookup_string("vertical-align","baseline"),
                })
            }
        };
        if looper.current_end + image_size.width > looper.extents.width {
            looper.adjust_current_line_vertical();
            looper.start_new_line();
            looper.add_box_to_current_line(bx);
        } else {
            looper.current_end += image_size.width;
            looper.add_box_to_current_line(bx);
        }
    }

    fn do_pre_layout(&self, looper:&mut Looper, txt:&str, link:&Option<String>) {
        let color = looper.style_node.lookup_color("color", &BLACK);
        let font_size = looper.style_node.lookup_font_size();
        // println!("font size is {:#?} ",font_size, color);
        let font_family = looper.style_node.lookup_font_family(looper.font_cache);

        let font_weight = looper.style_node.lookup_font_weight(400);
        let font_style = looper.style_node.lookup_string("font-style", "normal");
        let valign = looper.style_node.lookup_string("vertical-align", "baseline");
        for line in txt.split_terminator('\n') {
            let bounds = calculate_text_bounds(line, looper.font_cache, font_size, &font_family, font_weight, &font_style);
            if let Some(bounds) = bounds {
                let bx = RenderInlineBoxType::Text(RenderTextBox {
                    rect: Rect {
                        x: looper.current_start + bounds.min.x,
                        y: looper.current_bottom + bounds.min.y,
                        width: looper.extents.width,
                        height: font_size
                    },
                    text: line.to_string(),
                    color: Some(color.clone()),
                    font_size,
                    font_family: font_family.clone(),
                    link: link.clone(),
                    font_weight,
                    font_style:font_style.clone(),
                    valign:valign.clone(),
                });
                looper.add_box_to_current_line(bx);
                looper.current_bottom += looper.current.rect.height;
                looper.extents.height += looper.current.rect.height;
                looper.adjust_current_line_vertical();
                looper.start_new_line();
            }
        }
    }

    fn do_normal_inline_layout(&self, looper:&mut Looper, txt:&str, link:&Option<String>) {
        // println!("processing text '{}'", txt);
        let font_family = looper.style_node.lookup_font_family(looper.font_cache);
        // println!("using font family {}", font_family);
        let font_weight = looper.style_node.lookup_font_weight(400);
        let font_size = looper.style_node.lookup_font_size();
        let font_style = looper.style_node.lookup_string("font-style", "normal");
        let vertical_align = looper.style_node.lookup_string("vertical-align","baseline");
        let line_height = font_size;
        // let line_height = looper.style_node.lookup_length_px("line-height", line_height);
        let color = looper.style_node.lookup_color("color", &BLACK);
        // println!("text has fam={:#?} color={:#?} fs={} weight={} style={}",
        //          font_family, color, font_size, font_weight, font_style );
        // println!("styles={:#?}",looper.style_node);
        // println!("parent={:#?}", parent.get_style_node());
        let mut curr_text = String::new();
        for word in txt.split_whitespace() {
            let mut word2 = String::from(" ");
            word2.push_str(word);
            let w: f32 = calculate_word_length(word2.as_str(), looper.font_cache, font_size, &font_family, font_weight, &font_style);
            //if it's too long then we need to wrap
            if looper.current_end + w > looper.extents.width {
                //add current text to the current line
                // println!("wrapping: {} cb = {}", curr_text, looper.current_bottom);
                let bx = RenderInlineBoxType::Text(RenderTextBox{
                    rect: Rect{
                        x: looper.current_start,
                        y: looper.current_bottom,
                        width: looper.current_end - looper.current_start,
                        height: line_height
                    },
                    text: curr_text,
                    color: Some(color.clone()),
                    font_size,
                    font_family: font_family.clone(),
                    font_style: font_style.clone(),
                    link: link.clone(),
                    font_weight,
                    valign: vertical_align.clone(),
                });
                looper.add_box_to_current_line(bx);
                //make new current text with the current word
                curr_text = String::new();
                curr_text.push_str(&word2);
                curr_text.push_str(" ");
                looper.current_bottom += looper.current.rect.height;
                looper.extents.height += looper.current.rect.height;
                looper.adjust_current_line_vertical();
                looper.start_new_line();
                looper.current_end += w;
            } else {
                looper.current_end += w;
                curr_text.push_str(&word2);
            }
        }
        let bx = RenderInlineBoxType::Text(RenderTextBox{
            rect: Rect {
                x: looper.current_start,
                y: looper.current_bottom,
                width: looper.current_end - looper.current_start,
                height: line_height,
            },
            text: curr_text,
            color: Some(color.clone()),
            font_size,
            font_family,
            link: link.clone(),
            font_weight,
            font_style,
            valign: vertical_align.clone(),
        });
        looper.add_box_to_current_line(bx);
    }

    fn do_inline(&self, looper:&mut Looper) {
        // println!("doing inline {:#?}", &self.debug_calculate_element_name());
        let link:Option<String> = match &looper.style_node.node.node_type {
            Text(_) => None,
            NodeType::Comment(_) => None,
            NodeType::Cdata(_) => None,
            Element(ed) => {
                if ed.tag_name == "a" {
                    ed.attributes.get("href").map(String::from)
                } else {
                    None
                }
            },
            NodeType::Meta(_) => None,
        };
        if let BoxType::InlineNode(snode) = &self.box_type {
            match &snode.node.node_type {
                 NodeType::Text(txt) => {
                     let whitespace = looper.style_node.lookup_keyword("white-space", &Keyword(String::from("normal")));
                     // println!("laying out using whitespace {:#?}", whitespace);
                     match whitespace {
                         Keyword(str) => {
                             match &*str {
                                 "pre" => return self.do_pre_layout(looper,txt,&link),
                                 _ => return self.do_normal_inline_layout(looper,txt,&link),
                             }
                         },
                         _ => {
                             println!("invalid whitespace type");
                         }
                     }
                }
                //     if child is element
                NodeType::Element(_ed) => {
                    // println!("recursing");
                    let old = Rc::clone(&looper.style_node);
                    looper.style_node = Rc::clone(snode);
                    for ch in self.children.iter() {
                        ch.do_inline(looper);
                    }
                    looper.style_node =  old;
                }
                _ => {}
            }
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
        // println!("width set to {:#?}",width);
        //width percentage
        if let Length(per, Unit::Per) = width {
            // println!("its a percentage width {} {}",per,containing.content.width);
            width = Length(containing.content.width*(per/100.0), Px);
        }

        // margin, border, and padding have initial value of 0
        let zero = Length(0.0, Px);
        let mut margin_left = style.lookup("margin-left","margin", &zero);
        let mut margin_right = style.lookup("margin-right","margin", &zero);
        let border_left = style.lookup("border-width-left","border-width", &zero);
        let border_right = style.lookup("border-width-right","border-width", &zero);
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
        // println!("underflow = {}",underflow);

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
        // println!("final margin left is {:#?}",margin_left);
        // println!("width set to {:#?}",width);

        self.dimensions.content.width = self.length_to_px(&width);
        self.dimensions.padding.left = self.length_to_px(&padding_left);
        self.dimensions.padding.right = self.length_to_px(&padding_right);
        self.dimensions.border.left = self.length_to_px(&border_left);
        self.dimensions.border.right = self.length_to_px(&border_right);
        self.dimensions.margin.left = self.length_to_px_size(&margin_left, &width);
        self.dimensions.margin.right = self.length_to_px_size(&margin_right,&width);
        // println!("final width is width= {} padding = {} margin: {}",
        //          self.dimensions.content.width,
        //          self.dimensions.padding.left,
        //          self.dimensions.margin.left);
    }

    fn length_to_px_size(&self, value:&Value, dimension:&Value) -> f32 {
        match value {
            Length(v, Unit::Per) => self.length_to_px(dimension)*v/100.0,
            _ => self.length_to_px(value),
        }
    }
    fn length_to_px(&self, value:&Value) -> f32{
        let font_size = self.get_style_node().lookup_font_size();
        match value {
            Length(v, Unit::Px) => *v,
            Length(v, Unit::Em) => (*v)*font_size,
            Length(v, Unit::Rem) => (*v)*font_size, // TODO: use real document font size
            Length(_v, Unit::Per) => {
                println!("WARNING: percentage in length_to_px. should have be converted to pixels already");
                0.0
            }
            _ => {0.0}
        }
    }
    fn calculate_block_position(&mut self, containing: &mut Dimensions) {
        let zero = Length(0.0, Px);
        let style = self.get_style_node();
        //println!("caculating block position {:#?} border {:#?}",style, style.lookup("border-width-top","border-width",&zero));
        let margin = EdgeSizes {
            top: style.lookup_length_as_px("margin-top",0.0),
            bottom: style.lookup_length_as_px("margin-bottom",0.0),
            ..(self.dimensions.margin)
        };

        let border = EdgeSizes {
            top: style.lookup_length_as_px("border-width-top",0.0),
            bottom: style.lookup_length_as_px("border-width-bottom",0.0),
            ..(self.dimensions.border)
        };
        let padding = EdgeSizes {
            top: style.lookup_length_as_px("padding-top",0.0),
            bottom: style.lookup_length_as_px("padding-bottom",0.0),
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
        if let Some(val) = self.get_style_node().value("height") {
            self.dimensions.content.height = self.length_to_px(&val);
        }
    }

}

fn calculate_word_length(text:&str, fc:&mut FontCache, font_size:f32, font_family:&str, font_weight:i32, font_style:&str) -> f32 {
    let scale = Scale::uniform(font_size  as f32);
    fc.lookup_font(font_family,font_weight, font_style);
    let sec = Section {
        text,
        scale,
        ..Section::default()
    };
    let glyph_bounds = fc.brush.glyph_bounds(sec);
    match &glyph_bounds {
        Some(rect) => rect.max.x as f32 + FUDGE,
        None => 0.0,
    }
}
fn calculate_text_bounds(text:&str, fc:&mut FontCache, font_size:f32, font_family:&str, font_weight:i32, font_style:&str) -> Option<GBRect<f32>> {
    let scale = Scale::uniform(font_size  as f32);
    fc.lookup_font(font_family,font_weight, font_style);
    let sec = Section {
        text,
        scale,
        ..Section::default()
    };
    fc.brush.glyph_bounds(sec)
}

struct Looper<'a> {
    lines:Vec<RenderLineBox>,
    current: RenderLineBox,
    extents:Rect,
    current_start:f32,
    current_end:f32,
    current_bottom:f32,
    font_cache:&'a mut FontCache,
    doc: &'a Document,
    style_node: Rc<StyledNode>,
}

impl Looper<'_> {
    fn start_new_line(&mut self) {
        let old = mem::replace(&mut self.current, RenderLineBox {
            rect: Rect{
                x: self.extents.x,
                y: self.current_bottom,
                width: self.extents.width,
                height: 0.0
            },
            baseline:0.0,
            children: vec![],
        });
        self.lines.push(old);
        self.current_start = self.extents.x;
        self.current_end = self.extents.x;
    }
    fn add_box_to_current_line(&mut self, bx:RenderInlineBoxType) {
        let rect = match &bx {
            RenderInlineBoxType::Text(bx) => &bx.rect,
            RenderInlineBoxType::Error(bx) => &bx.rect,
            RenderInlineBoxType::Image(bx) => &bx.rect,
            RenderInlineBoxType::Block(bx) => &bx.rect,
        };
        self.current.rect.height = self.current.rect.height.max(rect.height);
        self.current.children.push(bx);
        self.current_start = self.current_end;
    }
    fn adjust_current_line_vertical(&mut self) {
        for ch in self.current.children.iter_mut() {
            let (mut rect, string) =  match ch {
                RenderInlineBoxType::Text(bx)    => (&mut bx.rect,&bx.valign),
                RenderInlineBoxType::Error(bx)  => (&mut bx.rect,&bx.valign),
                RenderInlineBoxType::Image(bx) => (&mut bx.rect,&bx.valign),
                RenderInlineBoxType::Block(bx)  => (&mut bx.rect,&bx.valign),
            };
            match string.as_str() {
                "bottom" => {
                    rect.y = self.current.rect.y + self.current.rect.height - rect.height;
                },
                "sub" => {
                    rect.y = self.current.rect.y + self.current.rect.height - rect.height - 10.0 + 10.0;
                },
                "baseline" => {
                    rect.y = self.current.rect.y + self.current.rect.height - rect.height - 10.0;
                },
                "super" => {
                    rect.y = self.current.rect.y + self.current.rect.height - rect.height - 10.0 - 10.0;
                },
                "middle" => {
                    rect.y = self.current.rect.y + (self.current.rect.height - rect.height)/2.0;
                },
                "top" => {
                    rect.y = self.current.rect.y;
                },
                _ => {}
            }
        }
    }

}

/*
#[test]
fn test_layout<'a>() {
    let mut font_cache = FontCache::new();
    font_cache.install_font("sans-serif",400.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font("sans-serif", 700.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());

    let doc = load_doc_from_net(&relative_filepath_to_url("tests/nested.html").unwrap()).unwrap();
    let ss_url = relative_filepath_to_url("tests/default.css").unwrap();
    let mut stylesheet = load_stylesheet_from_net(&ss_url).unwrap();
    font_cache.scan_for_fontface_rules(&stylesheet);
    let snode = style_tree(&doc.root_node,&stylesheet);
    println!(" ======== build layout boxes ========");
    let mut root_box = build_layout_tree(&snode, &doc);
    let mut containing_block = Dimensions {
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
    let _render_box = root_box.layout(&mut containing_block, &mut font_cache, &doc);
    // println!("final render box is {:#?}", render_box);
}
*/
fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
/*
#[test]
fn test_inline_block_element_layout() {
    let mut font_cache = FontCache::new();
    font_cache.install_font("sans-serif",400.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font("sans-serif", 700.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());{}

    let doc = load_doc_from_bytestring(b"<html><body><div><button>foofoo</button></div></body></html>");
    let ss_url = relative_filepath_to_url("tests/default.css").unwrap();
    let mut stylesheet = load_stylesheet_from_net(&ss_url).unwrap();
    font_cache.scan_for_fontface_rules(&stylesheet);
    let snode = style_tree(&doc.root_node,&stylesheet);
    let mut root_box = build_layout_tree(&snode, &doc);
    let mut containing_block = Dimensions {
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
    let _render_box = root_box.layout(&mut containing_block, &mut font_cache, &doc);
}
*/
/*
fn standard_init<'a,R:Resources,F:Factory<R>>(html:&[u8], css:&[u8]) -> (FontCache, Document, Stylesheet){
    let mut font_cache = FontCache::new();
    font_cache.install_font("sans-serif",400.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font("sans-serif", 700.0, "normal",
                            &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());{}
    let mut doc = load_doc_from_bytestring(html);
    let stylesheet = parse_stylesheet_from_bytestring(css).unwrap();
    let styled = style_tree(&doc.root_node,&stylesheet);
    let mut root_box = build_layout_tree(&styled, &doc);
    let mut cb = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };
    let render_box = root_box.layout(&mut cb, &mut font_cache, &doc);
    // println!("the final render box is {:#?}",render_box);
    return (font_cache,doc, stylesheet);
}
*/
/*
#[test]
fn test_table_layout() {
    let render_box = standard_init(
        br#"<table>
            <tbody>
                <tr>
                    <td>data 1</td>
                    <td>data 2</td>
                    <td>data 3</td>
                </tr>
                <tr>
                    <td>data 4</td>
                    <td>data 5</td>
                    <td>data 6</td>
                </tr>
                <tr>
                    <td>data 7</td>
                    <td>data 8</td>
                    <td>data 9</td>
                </tr>
            </tbody>
        </table>"#,
        br#"
        table {
            display: table;
        }
        tbody {
            display: table-row-group;
        }
        tr {
            display: table-row;
        }
        td {
            display: table-cell;
        }
        "#
    );
    println!("it all ran! {:#?}",render_box);
}
*/

pub enum Brush {
    Style1(glium_glyph::GlyphBrush<'static, 'static>),
    Style2(glium_glyph::glyph_brush::GlyphBrush<'static, Font<'static>>),
}
impl Brush {
    fn glyph_bounds(&mut self, sec:Section) -> Option<GBRect<f32>> {
        match self {
            Brush::Style1(b) => b.glyph_bounds(sec),
            Brush::Style2(b) => b.glyph_bounds(sec),
        }
    }
    pub fn queue(&mut self, sec:Section) {
        match self {
            Brush::Style1(b) => b.queue(sec),
            Brush::Style2(b) => b.queue(sec),
        }
    }
    pub fn draw_queued_with_transform(&mut self, mat:[[f32;4];4],
                                      facade:&glium::Display,
                                      frame:&mut glium::Frame) {
        match self {
            Brush::Style1(b) => b.draw_queued_with_transform(mat,facade,frame),
            Brush::Style2(_b) => {
                panic!("cant actuually draw with style two")
            },
        }
    }
}

fn standard_init(html:&[u8],css:&[u8]) -> Result<RenderBox,BrowserError> {

    let open_sans_light: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Light.ttf");
    let open_sans_reg: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Regular.ttf");
    let open_sans_bold: &[u8] = include_bytes!("../tests/fonts/Open_Sans/OpenSans-Bold.ttf");
    let doc = load_doc_from_bytestring(html);
    let mut stylesheet = parse_stylesheet_from_bytestring(css).unwrap();
    expand_styles(&mut stylesheet);
    let styled = dom_tree_to_stylednodes(&doc.root_node, &stylesheet);
    // println!("styled nodes {:#?}",styled);
    let glyph_brush:glium_glyph::glyph_brush::GlyphBrush<Font> =
        glium_glyph::glyph_brush::GlyphBrushBuilder::without_fonts().build();
    let mut viewport = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 0.0,
        },
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default()
    };
    let mut root_box = build_layout_tree(&styled.root.borrow(), &doc);
    let mut font_cache = FontCache {
        brush: Brush::Style2(glyph_brush),
        families: Default::default(),
        fonts: Default::default()
    };
    font_cache.install_font(Font::from_bytes(open_sans_light)?,"sans-serif",100, "normal");
    font_cache.install_font(Font::from_bytes(open_sans_reg)?,"sans-serif",400, "normal");
    font_cache.install_font(Font::from_bytes(open_sans_bold)?,"sans-serif",700, "normal");
    let render_box = root_box.layout(&mut viewport, &mut font_cache, &doc);
    Ok(render_box)
}

#[test]
fn test_insets() {
    let render_box = standard_init(
        br#"<body></body>"#,
        br#"body { display:block; margin: 50px; padding: 50px; border-width: 50px; } "#
    ).unwrap();
    println!("it all ran! {:#?}",render_box);
    match render_box {
        RenderBox::Block(bx) => {
            assert_eq!(bx.margin.left,50.0);
            assert_eq!(bx.padding.left,50.0);
            assert_eq!(bx.border_width.left,50.0);
        }
        _ => {
            panic!("this should have been a block box");
        }
    }
    // assert_eq!(render_box.calculate_insets().left,100);

}

#[test]
fn test_font_weight() {
    let render_box = standard_init(
        br#"<body>text</body>"#,
        br#"body { display:block; font-weight: bold; } "#
    ).unwrap();
    println!("it all ran! {:#?}",render_box);
}

#[test]
fn test_blue_text() {
    let render_box = standard_init(
        br#"<body><a>link</a></body>"#,
        br#" a { color: blue; } body { display: block; color: red; }"#
    ).unwrap();
    println!("it all ran! {:#?}",render_box);
/*
    match render_box {
        RenderBox::Block(bx) => {
            // bx.children[0].children
            assert_eq!(bx.margin.left,50.0);
            assert_eq!(bx.padding.left,50.0);
            assert_eq!(bx.border_width.left,50.0);
        }
        _ => {
            panic!("this should have been a block box");
        }
    }
    // assert_eq!(render_box.calculate_insets().left,100);
*/
}
#[test]
fn test_pre_code_text() {
    let render_box = standard_init(
        br#"<pre><code>for i in node.children().iter() {
    println!("this is some rust code");
}
</code></pre>
"#,
br#"pre {
    display:block;
    white-space: pre;
}
code {
    font-size: 20px;
    white-space: inherit;
}"#,
    ).unwrap();
    println!("pre code demo is {:#?}",render_box);
}

#[test]
fn test_unordered_listitem() {
    let render_box = standard_init(
        br#"<ul>
    <li>item one</li>
    <li>item two</li>
</ul>"#,
        br#"        ul {
        display:block;
            padding-left: 40px;
            border: 1px solid black;
            list-style-type: disc;
        }
        li {
            display: list-item;
            border: 1px solid red;
        }
"#,
    ).unwrap();
    println!("ul render is {:#?}",render_box);

}

#[test]
fn test_margin_em() {
    let render_box = standard_init(
        br#"<div>foo</div>"#,
        br#"div {
            display:block;
            margin-left: 2.5em;
            font-size: 20px;
        }"#,
    ).unwrap();
    println!("ul render is {:#?}",render_box);
    if let RenderBox::Block(rbx) = render_box {
        assert_eq!(rbx.rect.x,50.0);
        assert_eq!(rbx.rect.width,450.0);
        assert_eq!(rbx.font_size,20.0);
    }

}

#[test]
fn test_margin_percentage() {
    let render_box = standard_init(
        br#"<div>foo</div>"#,
        br#"div {
            display:block;
            margin-left: 50%;
            font-size: 20px;
        }"#,
    ).unwrap();
    println!("ul render is {:#?}",render_box);
    if let RenderBox::Block(rbx) = render_box {
        assert_eq!(rbx.rect.x,250.0);
        assert_eq!(rbx.rect.width,500.0);
        assert_eq!(rbx.font_size,20.0);
    }

}
