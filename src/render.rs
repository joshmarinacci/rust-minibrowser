use crate::css::{Color, RuleType, Stylesheet, Value};
use crate::layout::Brush;
use crate::net::{load_font_from_net, relative_filepath_to_url};
use glium_glyph::glyph_brush::rusttype::{Error, Font};
use glium_glyph::glyph_brush::FontId;
use glium_glyph::GlyphBrush;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use url::Url;

#[allow(dead_code)]
pub const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
pub const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
pub const RED: Color = Color {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};
#[allow(dead_code)]
pub const BLUE: Color = Color {
    r: 0,
    g: 0,
    b: 255,
    a: 255,
};
pub const AQUA: Color = Color {
    r: 0,
    g: 255,
    b: 255,
    a: 255,
};
pub const YELLOW: Color = Color {
    r: 255,
    g: 255,
    b: 0,
    a: 255,
};
#[allow(dead_code)]
pub const GREEN: Color = Color {
    r: 0,
    g: 255,
    b: 0,
    a: 255,
};
pub const MAGENTA: Color = Color {
    r: 255,
    g: 0,
    b: 255,
    a: 255,
};

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
    pub families: HashMap<String, String>,
    // names:HashMap<String,Url>,
    pub fonts: HashMap<String, FontId>,
    // default_font: Option<Font>,
}

impl FontCache {
    pub fn make_key(&self, family: &str, weight: i32, style: &str) -> String {
        return format!("{}-{}-{}", family, weight, style);
    }
    pub fn install_font(&mut self, font: Font<'static>, family: &str, weight: i32, style: &str) {
        let fid = match &mut self.brush {
            Brush::Style1(b) => b.add_font(font),
            Brush::Style2(b) => b.add_font(font),
        };
        let key = self.make_key(family, weight, style);
        // println!("installing font {}",key);
        self.fonts.insert(key, fid);
        self.families
            .insert(String::from(family), String::from(family));
    }
    pub fn lookup_font(&mut self, fam: &str, wt: i32, sty: &str) -> &FontId {
        // println!("looking up font {} {} {}", fam, wt, sty);
        let key = self.make_key(fam, wt, sty);
        self.fonts.get(&*key).unwrap()
    }
    pub fn has_font_family(&self, family: &str) -> bool {
        self.families.contains_key(family)
    }
}

fn find_truetype_url(value: &Value, url: &Url) -> Option<Url> {
    match value {
        Value::FunCall(fcv) => match &fcv.arguments[0] {
            Value::StringLiteral(str) => {
                if !str.to_lowercase().ends_with(".ttf") {
                    return None;
                }
                if let Ok(url) = url.join(str.as_str()) {
                    Some(url)
                } else {
                    println!("parsing error on url {:#?}", url);
                    None
                }
            }
            _ => None,
        },
        Value::ArrayValue(vals) => {
            for val in vals.iter() {
                let res = find_truetype_url(val, url);
                if res.is_some() {
                    return res;
                }
            }
            None
        }
        _ => None,
    }
}
fn extract_font_weight(value: &Value) -> Option<i32> {
    match value {
        Value::Keyword(str) => match str.as_str() {
            "normal" => Some(400),
            "bold" => Some(700),
            _ => None,
        },
        Value::Number(val) => Some((*val) as i32),
        _ => None,
    }
}

impl FontCache {
    pub fn scan_for_fontface_rules(&mut self, stylesheet: &Stylesheet) {
        for rule in stylesheet.rules.iter() {
            if let RuleType::AtRule(at_rule) = rule {
                if at_rule.name == "font-face" {
                    println!("found a font face rule");
                    // println!("we have an at rule {:#?}",at_rule);
                    for rule in at_rule.rules.iter() {
                        if let RuleType::Rule(rule) = &rule {
                            // println!("Processing real rules {:#?}",rule);
                            let mut src: Option<Url> = Option::None;
                            let mut font_family: Option<String> = Option::None;
                            let mut font_weight: Option<i32> = Option::None;
                            for dec in rule.declarations.iter() {
                                if dec.name == "src" {
                                    src = find_truetype_url(&dec.value, &stylesheet.base_url);
                                }
                                if dec.name == "font-weight" {
                                    font_weight = extract_font_weight(&dec.value);
                                }
                                if dec.name == "font-style" {}
                                if dec.name == "font-family" {
                                    match &dec.value {
                                        Value::StringLiteral(str) => {
                                            font_family = Some(str.clone())
                                        }
                                        _ => font_family = None,
                                    }
                                }
                            }
                            println!("got it {:#?} {:#?} {:#?}", font_family, src, font_weight);
                            if font_family.is_some() && src.is_some() && font_weight.is_some() {
                                let url = src.unwrap();
                                let font = load_font_from_net(url);
                                let font = font.unwrap();
                                self.install_font(
                                    font,
                                    &*font_family.unwrap(),
                                    font_weight.unwrap(),
                                    "normal",
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
