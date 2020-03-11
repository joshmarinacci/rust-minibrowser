use raqote::{DrawTarget,
    SolidSource, PathBuilder, Source,
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;
use crate::css::Color;
use crate::layout::{Rect, RenderBox, RenderInlineBoxType};
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use url::Url;
use crate::net::relative_filepath_to_url;

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
        width: width,
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

pub fn draw_render_box(root:&RenderBox, dt:&mut DrawTarget, font:&mut FontCache, viewport:&Rect) -> bool {
    // println!("====== rendering ======");
    match root {
        RenderBox::Block(block) => {
            match &block.background_color {
                Some(color) => {
                    let r = Rect {
                        x: block.rect.x - block.padding.left - block.border_width,
                        y: block.rect.y - block.padding.top - block.border_width,
                        width: block.rect.width + block.padding.left + block.padding.right + block.border_width*2.0,
                        height: block.rect.height + block.padding.top + block.padding.bottom + block.border_width*2.0
                    };
                    fill_rect(dt, &r, &render_color_to_source(color))
                },
                _ => {}
            }

            if block.border_width > 0.0 {
                match &block.border_color {
                    Some(color) => {
                        let r = Rect {
                            x: block.rect.x - block.padding.left - block.border_width,
                            y: block.rect.y - block.padding.top - block.border_width,
                            width: block.rect.width + block.padding.left + block.padding.right + block.border_width*2.0,
                            height: block.rect.height + block.padding.top + block.padding.bottom + block.border_width*2.0
                        };
                        stroke_rect(dt, &r, &render_color_to_source(color), block.border_width)
                    },
                    _ => {}
                }
            }
            // stroke_rect(dt, &block.rect, &render_color_to_source(&BLACK), 1 as f32);
            for ch in block.children.iter() {
                match ch {
                    RenderBox::Block(blk) => {
                        if (blk.rect.y > viewport.y + viewport.height) {
                            println!("outside! {}", blk.rect.y);
                            return false;
                        }
                    }
                    _ => {}
                }

                let ret = draw_render_box(&ch,dt,font, viewport);
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
                            // println!("text is {} {} {}", inline.rect.y, inline.rect.height, inline.text.trim());
                            let trimmed = text.text.trim();
                            if trimmed.len() > 0 {
                                let font = font.get_font(&String::from("sans-serif"));
                                match &text.color {
                                    Some(color) => draw_text(dt, font, &text.rect, &trimmed, &render_color_to_source(color), text.font_size),
                                    _ => {}
                                }
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
/*

store the font family on the layout box
at render time, grab the actual font from the font cache
if not in the font cache, then load into the cache, then return

*/

#[derive(Debug, Default)]
pub struct FontCache {
    pub names:HashMap<String,Url>,
    pub fonts:HashMap<String,Font>,
}
impl FontCache {
    pub fn install_font(&mut self, name:&String, url:&Url) {
        println!("installing the font {} at url {}",name,url);
        self.names.insert(name.clone(),url.clone());
    }
    pub fn get_font(&mut self, name:&String) -> &Font {
        if !self.fonts.contains_key(name) {
            self.load_font(name)
        }
        return self.fonts.get(name).unwrap();
    }
    fn load_font(&mut self, name:&String) {
        println!("trying to load the font: '{}'",name);
        let pth = self.names.get(name).unwrap().to_file_path().unwrap();
        let mut file = File::open(pth).unwrap();
        let font = Font::from_file(&mut file, 0).unwrap();
        self.fonts.insert(String::from(name), font);
    }
}

static TEST_FONT_FILE_PATH: &'static str =
    "tests/tufte/et-book/et-book-roman-line-figures/et-book-roman-line-figures.ttf";
#[test]
fn test_font_loading() {
    let pth = Path::new(TEST_FONT_FILE_PATH);
    let mut file = File::open(pth).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let mut fc = FontCache{
        names: HashMap::new(),
        fonts: HashMap::new()
    };
    let name = String::from("sans-serif");
    fc.install_font(&name, &relative_filepath_to_url(TEST_FONT_FILE_PATH).unwrap());
    println!("{:#?}",fc.get_font(&String::from("sans-serif")));
    // println!("{:#?}",fc);
    //assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

