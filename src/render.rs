use raqote::{DrawTarget,
    SolidSource, PathBuilder, Source,
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;
use crate::css::Color;
use crate::layout::{LayoutBox, Dimensions, Rect};
use crate::layout::BoxType::BlockNode;


#[derive(Debug)]
pub struct Point {
    pub x:f32,
    pub y:f32,
}

#[derive(Debug)]
pub struct Size {
    pub w:f32,
    pub h:f32,
}

#[derive(Debug)]
pub struct Inset {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Inset {
    pub fn empty() -> Inset {
        Inset {
            top:0.,
            right:0.,
            bottom:0.,
            left:0.,
        }
    }
    pub fn same(v:f32) -> Inset {
        Inset {
            top: v,
            right: v,
            bottom: v,
            left: v
        }
    }
}

#[allow(dead_code)]
pub const BLACK:Color = Color { r:0, g:0, b:0, a:255 };
pub const WHITE:Color = Color { r:255, g:255, b:255, a:255 };
pub const RED:Color = Color { r:255, g:0, b:0, a:255 };
#[allow(dead_code)]
pub const BLUE:Color = Color { r:0, g:0, b:255, a:255 };
#[allow(dead_code)]
pub const GREEN:Color = Color { r:0, g:255, b:0, a:255 };

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

// fn draw_text(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str, color:&Color) {
//     let c = render_color_to_source(color);
//     dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32), &c, &DrawOptions::new(),);
// }

fn render_color_to_source(c:&Color) -> Source {
    return Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b));
}

pub fn draw_block_box(dt:&mut DrawTarget, bb:&LayoutBox, font:&Font) {
    // println!("drawing a block node {} {}", bb.dimensions.content.width, bb.dimensions.content.height);
    fill_rect(dt,&bb.dimensions.content, &render_color_to_source(&BLUE));
    stroke_rect(dt,&bb.dimensions.content, &render_color_to_source(&GREEN), bb.dimensions.border.left);

    for child in bb.children.iter() {
        match &child.box_type {
            BlockNode(node) => {
                draw_block_box(dt, child,font);
            }
            InlineNode => {
                println!("drawing an inline node")
            }
            AnonymousBlock => {
                //println!("doing an anonymous block")
            }

            // RenderBox::Line(text) => {
            //     draw_text(dt, &font, &text.pos, &text.text, &text.color);
            // }
        }
    }
}


