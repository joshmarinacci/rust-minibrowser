use raqote::{DrawTarget,
    SolidSource, PathBuilder, Source,
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;
use crate::css::Color;
use crate::layout::{LayoutBox, Dimensions, Rect, RenderBox};
use crate::layout::BoxType::BlockNode;

#[allow(dead_code)]
pub const BLACK:Color = Color { r:0, g:0, b:0, a:255 };
pub const WHITE:Color = Color { r:255, g:255, b:255, a:255 };
pub const RED:Color = Color { r:255, g:0, b:0, a:255 };
#[allow(dead_code)]
pub const BLUE:Color = Color { r:0, g:0, b:255, a:255 };
pub const AQUA:Color = Color { r:0, g:255, b:255, a:255 };
#[allow(dead_code)]
pub const GREEN:Color = Color { r:0, g:255, b:0, a:255 };
pub const PURPLE:Color = Color { r:255, g:0, b:255, a:255 };

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

fn draw_text(dt: &mut DrawTarget, font:&Font, rect:&Rect, text:&str, c:&Source) {
    dt.draw_text(font, 18.,
                 text,
                 raqote::Point::new(rect.x as f32, rect.y+rect.height as f32),
                 &c, &DrawOptions::new(),);
}

fn render_color_to_source(c:&Color) -> Source {
    return Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b));
}

pub fn draw_render_box(root:&RenderBox, dt:&mut DrawTarget, font:&Font) {
    // println!("====== rendering ======");
    match root {
        RenderBox::Block(block) => {
            // stroke_rect(dt, &block.rect, &render_color_to_source(&GREEN), 1 as f32);
            for ch in block.children.iter() {
                draw_render_box(&ch,dt,font);
            }
        },
        RenderBox::Inline() => {

        },
        RenderBox::Anonymous(block) => {
            //don't draw anonymous blocks that are empty
            if block.children.len() <= 0 {
                return;
            }
            stroke_rect(dt, &block.rect, &render_color_to_source(&RED), 1 as f32);
            for line in block.children.iter() {
                stroke_rect(dt, &line.rect, &render_color_to_source(&AQUA), 1 as f32);
                for inline in line.children.iter() {
                    stroke_rect(dt, &inline.rect, &render_color_to_source(&PURPLE), 1 as f32);
                    // println!("text is {} {} {}", inline.rect.y, inline.rect.height, inline.text.trim());
                    let trimmed = inline.text.trim();
                    if trimmed.len() > 0 {
                        draw_text(dt, font, &inline.rect, &trimmed, &render_color_to_source(&BLACK));
                    }
                }
            }
        }
    }
}


