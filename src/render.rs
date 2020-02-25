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

pub fn drawRect(dt: &mut DrawTarget, pos:&Point, size:&Size, color:&Source) {
    let mut pb = PathBuilder::new();
    pb.rect(pos.x as f32, pos.y as f32, size.w as f32, size.h as f32);
    let path = pb.finish();
    dt.fill(&path, 
        color, 
        &DrawOptions::new());
}

fn drawText(dt: &mut DrawTarget, font:&Font, pos:&Point, text:&str) {
    dt.draw_text(font, 36., text, raqote::Point::new(pos.x as f32,pos.y as f32),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)),
        &DrawOptions::new(),
   );    
}

pub fn drawBlockBox(dt:&mut DrawTarget, bb:&BlockBox, font:&Font) {
    let GREEN:Source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0xff, 0));
    drawRect(dt,&bb.pos, &bb.size, &GREEN);

    for child in bb.boxes.iter() {
        match child {
            RenderBox::Block(block) => {
                drawBlockBox(dt, &block, font);
            },
            RenderBox::Line(text) => {
                drawText(dt, &font, &text.pos, &text.text);
            }
        }
    }
}
