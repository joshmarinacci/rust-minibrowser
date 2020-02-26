use raqote::{DrawTarget, SolidSource, PathBuilder, Source, DrawOptions};
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
}
pub struct LineBox {
    pub pos:Point,
    pub text:String,
}

pub fn draw_rect(dt: &mut DrawTarget, pos:&Point, size:&Size, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    dt.fill(&path, 
        color, 
        &DrawOptions::new());
}

fn draw_text(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str) {
    dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)),
        &DrawOptions::new(),
   );    
}

pub fn draw_block_box(dt:&mut DrawTarget, bb:&BlockBox, font:&Font) {
    let green:Source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0xff, 0));
    draw_rect
(dt,&bb.pos, &bb.size, &green);

    for child in bb.boxes.iter() {
        match child {
            RenderBox::Block(block) => {
                draw_block_box(dt, &block, font);
            },
            RenderBox::Line(text) => {
                draw_text(dt, &font, &text.pos, &text.text);
            }
        }
    }
}


pub fn drawRenderBox(dt:&mut DrawTarget, rb:&RenderBox, font:&Font) {
    match rb {
        RenderBox::Block(block) => {
            drawBlockBox(dt, &block, font);
        },
        RenderBox::Line(line) => {
            drawText(dt, &font, &line.pos, &line.text);
        }
    }
}