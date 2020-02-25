use minifb::{MouseMode, Window, WindowOptions, ScaleMode, Scale};
use raqote::{DrawTarget, SolidSource, Source, DrawOptions, PathBuilder, Point, Transform, StrokeStyle};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
const WIDTH: usize = 400;
const HEIGHT: usize = 400;

fn main() {
    let mut window = Window::new("Raqote", WIDTH, HEIGHT, WindowOptions {
                                    ..WindowOptions::default()
                                }).unwrap();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    let size = window.get_size();
    let mut dt = DrawTarget::new(size.0 as i32, size.1 as i32);
    loop {
        dt.clear(SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff));
        let mut pb = PathBuilder::new();
        if let Some(pos) = window.get_mouse_pos(MouseMode::Clamp) {

            pb.rect(pos.0, pos.1, 100., 130.);
            let path = pb.finish();
            dt.fill(&path, &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0xff, 0)), &DrawOptions::new());

            let pos_string = format!("{:?}", pos);
            dt.draw_text(&font, 36., &pos_string, Point::new(0., 100.),
                         &Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0)),
                         &DrawOptions::new(),
                        );

            window.update_with_buffer(dt.get_data(), size.0, size.1).unwrap();
        }
    }
}
