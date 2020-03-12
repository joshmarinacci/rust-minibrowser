use raqote::{DrawTarget,
    SolidSource, PathBuilder, Source,
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;
use crate::css::{Color, Value, Stylesheet, RuleType};
use crate::layout::{Rect, RenderBox, RenderInlineBoxType};
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use url::Url;
use crate::net::relative_filepath_to_url;
use crate::css::Value::Keyword;
use font_kit::source::SystemSource;

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

fn render_color_to_source(c:&Color) -> Source {
    return Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b));
}

pub fn draw_render_box(root:&RenderBox, dt:&mut DrawTarget, font_cache:&mut FontCache, viewport:&Rect) -> bool {
    // println!("====== rendering ======");
    match root {
        RenderBox::Block(block) => {
            match &block.background_color {
                Some(color) => fill_rect(dt, &block.content_area_as_rect(), &render_color_to_source(color)),
                _ => {}
            }

            if block.border_width > 0.0 && block.border_color.is_some() {
                let color = render_color_to_source(&block.border_color.as_ref().unwrap());
                stroke_rect(dt, &block.content_area_as_rect(), &color, block.border_width)
            }
            // stroke_rect(dt, &block.rect, &render_color_to_source(&BLACK), 1 as f32);
            for ch in block.children.iter() {
                match ch {
                    RenderBox::Block(blk) => {
                        if blk.rect.y > viewport.y + viewport.height {
                            println!("outside! {}", blk.rect.y);
                            return false;
                        }
                    }
                    _ => {}
                }

                let ret = draw_render_box(&ch, dt, font_cache, viewport);
                if ret == false {
                    return false;
                }
            }
            return true;
        },
        RenderBox::Inline() => {
            return true;
        },
        RenderBox::InlineBlock() => {
            return true;
        },
        RenderBox::Anonymous(block) => {
            //don't draw anonymous blocks that are empty
            if block.children.len() <= 0 {
                return true;
            }
            // stroke_rect(dt, &block.rect, &render_color_to_source(&RED), 1 as f32);
            for line in block.children.iter() {
                // stroke_rect(dt, &line.rect, &render_color_to_source(&AQUA), 1 as f32);
                for inline in line.children.iter() {
                    match inline {
                        RenderInlineBoxType::Text(text) => {
                            // stroke_rect(dt, &text.rect, &render_color_to_source(&MAGENTA), 1 as f32);
                            let trimmed = text.text.trim();
                            if text.color.is_some() && trimmed.len() > 0 {
                                let font = font_cache.get_font(&text.font_family, text.font_weight);
                                draw_text(dt, font, &text.rect, &trimmed, &render_color_to_source(&text.color.as_ref().unwrap()), text.font_size);
                            }
                        }
                        RenderInlineBoxType::Image(img) => {
                            dt.draw_image_at(img.rect.x,img.rect.y,&img.image.to_image(), &DrawOptions::default());
                        }
                        RenderInlineBoxType::Error(err) => {
                            fill_rect(dt, &err.rect, &render_color_to_source(&MAGENTA))
                        }
                    }
                }
            }
            return true;
        }
    }
}

#[derive(Debug, Default)]
pub struct FontCache {
    families:HashMap<String,Url>,
    names:HashMap<String,Url>,
    fonts:HashMap<String,Font>,
}

fn extract_url(value:&Value, url:&Url) -> Option<Url> {
    match value {
        Value::FunCall(fcv) => {
            match &fcv.arguments[0] {
                Value::StringLiteral(str) => {
                    let url = url.join(str.as_str());
                    if url.is_ok() {
                        Some(url.unwrap())
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

impl FontCache {
    pub fn new() -> Self {
        Self {
            families: HashMap::new(),
            names: HashMap::new(),
            fonts: HashMap::new()
        }
    }
    pub fn has_font_family(&self, name:&String) -> bool {
        return self.families.contains_key(name);
    }
    pub fn install_font(&mut self, name:&String, weight:f32, url:&Url) {
        let key = format!("{}-{:#?}",name,weight);
        println!("installing the font {} {} at url {} {}",name,weight, url, key);

        let pth = url.to_file_path().unwrap();
        let mut file = File::open(pth).unwrap();
        let font = Font::from_file(&mut file, 0).unwrap();
        self.families.insert(name.clone(),url.clone());
        self.names.insert(key.clone(),url.clone());
        self.fonts.insert(key.clone(), font);
    }
    pub fn install_font_font(&mut self, name:&String, font:Font) {
        self.fonts.insert(name.clone(),font);
    }
    pub fn get_font(&mut self, name:&String, weight:f32) -> &Font {
        let key = format!("{}-{:#?}",name,weight);
        return self.fonts.get(&key).unwrap();
    }
    fn load_font(&mut self, name:&String) {
        println!("trying to load the font: '{}'",name);
        let pth = self.names.get(name).unwrap().to_file_path().unwrap();
        let mut file = File::open(pth).unwrap();
        let font = Font::from_file(&mut file, 0).unwrap();
        self.fonts.insert(String::from(name), font);
    }
    pub fn scan_for_fontface_rules(&mut self, stylesheet:&Stylesheet) {
        for rule in stylesheet.rules.iter() {
            match rule {
                RuleType::AtRule(at_rule) => {
                    if at_rule.name == "font-face" {
                        // println!("we have an at rule {:#?}",at_rule);
                        for rule in at_rule.rules.iter() {
                            match &rule {
                                RuleType::Rule(rule) => {
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
                                    println!("got it {:#?} {:#?} {:#?}",font_family, src, font_weight);
                                    if font_family.is_some() && src.is_some() && font_weight.is_some() {
                                        self.install_font(&font_family.unwrap(),
                                                                font_weight.unwrap(),
                                                                &src.unwrap()
                                        )
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

    }
}

static TEST_FONT_FILE_PATH: &'static str =
    "tests/tufte/et-book/et-book-roman-line-figures/et-book-roman-line-figures.ttf";
#[test]
fn test_font_loading() {
    let pth = Path::new(TEST_FONT_FILE_PATH);
    let mut file = File::open(pth).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let mut fc = FontCache::new();
    let name = String::from("sans-serif");
    fc.install_font(&name, 400.0, &relative_filepath_to_url(TEST_FONT_FILE_PATH).unwrap());
    println!("{:#?}",fc.get_font(&String::from("sans-serif"), 400.0));
    // println!("{:#?}",fc);
    //assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

