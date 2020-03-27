use crate::css::{Color, Value, Stylesheet, RuleType};
use crate::layout::{Rect, RenderBox, RenderInlineBoxType, RenderBlockBox, Brush, RenderTextBox};
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use url::Url;
use crate::net::relative_filepath_to_url;
use glium_glyph::GlyphBrush;
use glium_glyph::glyph_brush::rusttype::{Font};
use glium_glyph::glyph_brush::FontId;


#[allow(dead_code)]
pub const BLACK:Color = Color { r:0, g:0, b:0, a:255 };
pub const WHITE:Color = Color { r:255, g:255, b:255, a:255 };
pub const RED:Color = Color { r:255, g:0, b:0, a:255 };
#[allow(dead_code)]
pub const BLUE:Color = Color { r:0, g:0, b:255, a:255 };
pub const AQUA:Color = Color { r:0, g:255, b:255, a:255 };
pub const YELLOW:Color = Color { r:255, g:255, b:0, a:255 };
#[allow(dead_code)]
pub const GREEN:Color = Color { r:0, g:255, b:0, a:255 };
pub const MAGENTA:Color = Color { r:255, g:0, b:255, a:255 };

/*
pub fn fill_rect(dt: &mut DrawTarget, dim:&Rect, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(dim.x as f32, dim.y as f32, dim.width as f32, dim.height as f32);
    let path = pb.finish();
    dt.fill(&path, color, &DrawOptions::new());
 }

pub fn stroke_rect(dt: &mut DrawTarget, pos:&Rect, color:&Source, width:f32) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, pos.width as f32, pos.height as f32);
    let path = pb.finish();
    let default_stroke_style = StrokeStyle {
        cap: LineCap::Square,
        join: LineJoin::Miter,
        width,
        miter_limit: 2.,
        dash_array: vec![],
        dash_offset: 16.,
    };
    dt.stroke(&path, color, &default_stroke_style,  &DrawOptions::new());
}

fn draw_text(dt: &mut DrawTarget, font:&Font, rect:&Rect, text:&str, c:&Source, font_size:f32) {
    // println!("drawing text '{}', size {}, with font {:#?}",text, font_size, font.postscript_name());
    dt.draw_text(font, font_size,
                 text,
                 raqote::Point::new(rect.x as f32, rect.y+rect.height as f32),
                 &c, &DrawOptions::new(),);
}

fn color_to_source(c:&Color) -> Source {
    Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b))
}*/

/*
fn draw_render_box_block<R:Resources,F:Factory<R>>(block:&RenderBlockBox, dt:&mut DrawTarget, font_cache:&mut FontCache<R,F>, viewport:&Rect) -> bool {
    if let Some(color) = &block.background_color {
        fill_rect(dt, &block.content_area_as_rect(), &color_to_source(color));
    }

    if block.border_width > 0.0 && block.border_color.is_some() {
        let color = color_to_source(&block.border_color.as_ref().unwrap());
        stroke_rect(dt, &block.content_area_as_rect(), &color, block.border_width)
    }
    // stroke_rect(dt, &block.rect, &color_to_source(&BLACK), 1 as f32);
    for ch in block.children.iter() {
        if let RenderBox::Block(blk) = ch {
            if blk.rect.y > viewport.y + viewport.height {
                // println!("outside! {}", blk.rect.y);
                return false;
            }
        }

        let ret = draw_render_box(&ch, dt, font_cache, viewport);
        if !ret {
            return false;
        }
    }
    true
}
*/
/*
pub fn draw_render_box<R:Resources,F:Factory<R>>(root:&RenderBox, dt:&mut DrawTarget, font_cache:&mut FontCache<R,F>, viewport:&Rect) -> bool {
    // println!("====== rendering ======");
    match root {
        RenderBox::Block(block) => draw_render_box_block(block,dt,font_cache,viewport),
        RenderBox::Inline() => {   true    },
        RenderBox::InlineBlock() => {  true },
        RenderBox::Anonymous(block) => {
            //don't draw anonymous blocks that are empty
            if block.children.is_empty() {
                return true;
            }
            // stroke_rect(dt, &block.rect.with_inset(2.0), &color_to_source(&RED), 1 as f32);
            for line in block.children.iter() {
                // stroke_rect(dt, &line.rect.with_inset(4.0), &color_to_source(&AQUA), 1 as f32);
                for inline in line.children.iter() {
                    match inline {
                        RenderInlineBoxType::Text(text) => {
                            // stroke_rect(dt, &text.rect.with_inset(6.0), &color_to_source(&MAGENTA), 1 as f32);
                            if text.color.is_some() && !text.text.is_empty() {
                                let font = font_cache.get_font(&text.font_family, text.font_weight, &text.font_style);
                                draw_text(dt, font, &text.rect, &text.text, &color_to_source(&text.color.as_ref().unwrap()), text.font_size);
                            }
                        }
                        RenderInlineBoxType::Image(img) => {
                            dt.draw_image_at(img.rect.x,img.rect.y,&img.image.to_image(), &DrawOptions::default());
                        }
                        RenderInlineBoxType::Error(err) => {
                            fill_rect(dt, &err.rect, &color_to_source(&MAGENTA))
                        }
                        RenderInlineBoxType::Block(block) => {
                            draw_render_box_block(block,dt, font_cache, viewport);
                        }
                    }
                }
            }
            true
        }
    }
}
*/



pub struct FontCache {
    pub brush: Brush,
    // families:HashMap<String,Url>,
    // names:HashMap<String,Url>,
    pub fonts:HashMap<String,FontId>,
    // default_font: Option<Font>,
}

impl FontCache {
    pub fn make_key(&self, family:&str, weight:i32, style:&str) -> String{
        return format!("{}-{}-{}",family,weight,style);
    }
    pub fn install_font(&mut self, font:Font<'static>, family:&str, weight:i32, style:&str) {
        let fid = match &mut self.brush {
            Brush::Style1(b) => b.add_font(font),
            Brush::Style2(b) => b.add_font(font),
        };
        let key = self.make_key(family,weight,style);
        self.fonts.insert(key,fid);
    }
    pub fn lookup_font(&mut self, text:&RenderTextBox) -> &FontId {
        // println!("looking up font {} {} {}", text.font_family, text.font_weight, text.font_style);
        let key = self.make_key(&text.font_family, text.font_weight, &text.font_style);
        return self.fonts.get(&*key).unwrap();
    }
}

fn extract_url(value:&Value, url:&Url) -> Option<Url> {
    match value {
        Value::FunCall(fcv) => {
            match &fcv.arguments[0] {
                Value::StringLiteral(str) => {
                    if let Ok(url) = url.join(str.as_str()) {
                        Some(url)
                    } else {
                        println!("parsing error on url {:#?}", url);
                        None
                    }
                },
                _ => None,
            }
        }
        _ => None,
    }
}
fn extract_font_weight(value:&Value) -> Option<f32> {
    match value {
        Value::Keyword(str) => {
            match str.as_str() {
                "normal" => Some(400.0),
                "bold" => Some(700.0),
                _ => None,
            }
        },
        Value::Number(val) => Some(*val),
        _ => None,
    }
}
/*
impl FontCache {
    pub fn new() -> Self {
        Self {
            families: HashMap::new(),
            names: HashMap::new(),
            fonts: HashMap::new(),
            default_font: None,
        }
    }
    fn make_key(&self, name:&str, weight:f32, style:&str) -> String {
        format!("{}-{}-{}",name,weight,style)
    }
    pub fn has_font_family(&self, name:&str) -> bool {
        self.families.contains_key(name)
    }
    pub fn install_default_font(&mut self, name:&str, weight:f32, style:&str, url:&Url) {
        let key = self.make_key(name,weight,style);
        println!("installing the default font {} at url {}",key,url);

        let pth = url.to_file_path().unwrap();
        let mut file = File::open(pth).unwrap();
        let font = Font::from_file(&mut file, 0).unwrap();
        self.default_font = Some(font);
    }
    pub fn install_font(&mut self, name:&str, weight:f32, style:&str, url:&Url) {
        let key = self.make_key(name,weight,style);
        println!("installing the font {} at url {}",key,url);

        let pth = url.to_file_path().unwrap();
        let mut file = File::open(pth).unwrap();
        let font = Font::from_file(&mut file, 0).unwrap();
        self.families.insert(name.to_string(),url.clone());
        self.names.insert(key.clone(),url.clone());
        self.fonts.insert(key, font);
    }
    pub fn install_font_font(&mut self, name:&str, weight:f32, style:&str, font:Font) {
        let key = self.make_key(name,weight,style);
        println!("installing the font {}, {} from {:#?}",name,key,font);
        self.fonts.insert(key,font);
    }
    pub fn get_font(&mut self, name:&str, weight:f32, style:&str) -> &Font {
        let key = self.make_key(name,weight,style);
        // println!("fetching the font {}",key);
        if let Some(font) = self.fonts.get(&key) {
            return font;
        } else if let Some(font) = &self.default_font {
            return font
        } else {
            panic!("no default font set!");
        }
    }
    // fn load_font(&mut self, name:&str) {
    //     println!("trying to load the font: '{}'",name);
    //     let pth = self.names.get(name).unwrap().to_file_path().unwrap();
    //     let mut file = File::open(pth).unwrap();
    //     let font = Font::from_file(&mut file, 0).unwrap();
    //     self.fonts.insert(String::from(name), font);
    // }
    pub fn scan_for_fontface_rules(&mut self, stylesheet:&Stylesheet) {
        for rule in stylesheet.rules.iter() {
            if let RuleType::AtRule(at_rule) = rule {
                    if at_rule.name == "font-face" {
                        // println!("we have an at rule {:#?}",at_rule);
                        for rule in at_rule.rules.iter() {
                            if let RuleType::Rule(rule) = &rule {
                                // println!("Processing real rules {:#?}",rule);
                                let mut src:Option<Url> = Option::None;
                                let mut font_family:Option<String> = Option::None;
                                let mut font_weight:Option<f32> = Option::None;
                                for dec in rule.declarations.iter() {
                                    if dec.name == "src" {
                                        src = extract_url(&dec.value, &stylesheet.base_url);
                                    }
                                    if dec.name == "font-weight" {
                                        font_weight = extract_font_weight(&dec.value);
                                    }
                                    if dec.name == "font-family" {
                                        match &dec.value {
                                            Value::StringLiteral(str) => font_family = Some(str.clone()),
                                            _ => font_family = None,
                                        }
                                    }
                                }
                                // println!("got it {:#?} {:#?} {:#?}",font_family, src, font_weight);
                                if font_family.is_some() && src.is_some() && font_weight.is_some() {
                                    self.install_font(&font_family.unwrap(),
                                                            font_weight.unwrap(),
                                                            "normal",
                                                            &src.unwrap()
                                    )
                                }
                            }
                        }
                    }
                }
        }

    }
}
*/
/*
static TEST_FONT_FILE_PATH: &str =
    "tests/tufte/et-book/et-book-roman-line-figures/et-book-roman-line-figures.ttf";
#[test]
fn test_font_loading() {
    let pth = Path::new(TEST_FONT_FILE_PATH);
    let mut file = File::open(pth).unwrap();
    let _font = Font::from_file(&mut file, 0).unwrap();
    let mut fc = FontCache::new();
    let name = String::from("sans-serif");
    fc.install_font(&name, 400.0, "normal",&relative_filepath_to_url(TEST_FONT_FILE_PATH).unwrap());
    println!("{:#?}",fc.get_font("sans-serif", 400.0, "normal"));
    // println!("{:#?}",fc);
    //assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

*/
