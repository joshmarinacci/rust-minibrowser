use raqote::{DrawTarget,
    SolidSource, PathBuilder, Source,
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;
use crate::css::Color;
use crate::layout::{Rect, RenderBox, RenderInlineBoxType};

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
    dt.draw_text(font, font_size,
                 text,
                 raqote::Point::new(rect.x as f32, rect.y+rect.height as f32),
                 &c, &DrawOptions::new(),);
}

fn render_color_to_source(c:&Color) -> Source {
    return Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b));
}

pub fn draw_render_box(root:&RenderBox, dt:&mut DrawTarget, font:&Font, viewport:&Rect) {
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
                draw_render_box(&ch,dt,font, viewport);
            }
        },
        RenderBox::Inline() => {

        },
        RenderBox::InlineBlock() => {

        },
        RenderBox::Anonymous(block) => {
            //don't draw anonymous blocks that are empty
            if block.children.len() <= 0 {
                return;
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
        }
    }
}


