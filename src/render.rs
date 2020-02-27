use raqote::{DrawTarget, 
    SolidSource, PathBuilder, Source, 
    DrawOptions, StrokeStyle,
    LineCap, LineJoin
};
use font_kit::font::Font;


pub struct Point {
    pub x:i32,
    pub y:i32,
}

pub struct Size {
    pub w:i32,
    pub h:i32,
}

pub enum RenderBox {
    Block(BlockBox),
    Line(LineBox),
}

pub struct BlockBox {
    pub pos:Point,
    pub size:Size,
    pub boxes:Vec<RenderBox>,
    pub background_color:RenderColor,
    pub border_color:RenderColor,
}

pub struct LineBox {
    pub pos:Point,
    pub text:String,
    pub color:RenderColor,
}

pub struct RenderColor {
    pub r:u8,
    pub g:u8,
    pub b:u8,
    pub a:u8
}


#[allow(dead_code)]
pub const BLACK:RenderColor = RenderColor { r:0, g:0, b:0, a:255 };
pub const WHITE:RenderColor = RenderColor { r:255, g:255, b:255, a:255 };
pub const RED:RenderColor = RenderColor { r:255, g:0, b:0, a:255 };
pub const BLUE:RenderColor = RenderColor { r:0, g:0, b:255, a:255 };
#[allow(dead_code)]
pub const GREEN:RenderColor = RenderColor { r:0, g:255, b:0, a:255 };

pub fn fill_rect(dt: &mut DrawTarget, pos:&Point, size:&Size, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    dt.fill(&path, color, &DrawOptions::new());
}

pub fn stroke_rect(dt: &mut DrawTarget, pos:&Point, size:&Size, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    let default_stroke_style = StrokeStyle {
        cap: LineCap::Square,
        join: LineJoin::Bevel,
        width: 1.,
        miter_limit: 2.,
        dash_array: vec![],
        dash_offset: 16.,
    };
    dt.stroke(&path, color, &default_stroke_style,  &DrawOptions::new());
}

fn draw_text(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str, color:&RenderColor) {
    let c = render_color_to_source(color);
    dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32), &c, &DrawOptions::new(),);    
}

fn render_color_to_source(c:&RenderColor) -> Source {
    return Source::Solid(SolidSource::from_unpremultiplied_argb(c.a, c.r, c.g, c.b));
}

pub fn draw_block_box(dt:&mut DrawTarget, bb:&BlockBox, font:&Font) {
    fill_rect(dt,&bb.pos, &bb.size, &render_color_to_source(&bb.background_color));
    stroke_rect(dt,&bb.pos, &bb.size, &render_color_to_source(&bb.border_color));

    for child in bb.boxes.iter() {
        match child {
            RenderBox::Block(block) => {
                draw_block_box(dt, &block, font);
            },
            RenderBox::Line(text) => {
                draw_text(dt, &font, &text.pos, &text.text, &text.color);
            }
        }
    }
}


