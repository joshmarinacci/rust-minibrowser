extern crate image;
use image::{GenericImageView, DynamicImage};
use raqote::{Image, DrawTarget, PathBuilder, Gradient, GradientStop, Color, Point, Spread, DrawOptions, Source, SolidSource};
use std::fmt::{Debug, Formatter, Error, Display};
use std::fmt;

const WHITE_SOURCE: Source = Source::Solid(SolidSource {
    r: 0xff,
    g: 0xff,
    b: 0xff,
    a: 0xff,
});

#[derive(Debug,PartialEq)]
pub struct LoadedImage {
    path:String,
    width: i32,
    height: i32,
    data:Vec<u32>,
}

impl LoadedImage {
    pub(crate) fn to_image(&self) -> Image {
        Image {
            width: self.width as i32,
            height: self.height as i32,
            data: &self.data,
        }
    }
}
impl fmt::Display for LoadedImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "foo")
    }
}

pub fn load_image_from_path(path:&str) -> LoadedImage {
    let img = image::open(path).unwrap();
    let (w,h) = img.dimensions();

    let mut loaded = LoadedImage {
        path: path.to_string(),
        width: w as i32,
        height: h as i32,
        data: vec![255 as u32;(w*h) as usize]
    };
    // let mut data2 = vec![255 as u32;(w*h) as usize];
    for (x,y,pixel) in img.pixels() {
        let n = ((y*w+x)) as usize;
        loaded.data[n]
            = 0xFF000000
            | ((pixel[0] as u32)<<16)
            | ((pixel[1] as u32)<< 8)
            | ((pixel[2] as u32)<< 0)
        ;
    }
    return loaded;
}
#[test]
fn test_image_load() {
    let image  = load_image_from_path("tests/images/cat.jpg");
    let mut dt = DrawTarget::new(image.width, image.height);
    dt.draw_image_at(0.,0., &image.to_image(), &DrawOptions::default());
    dt.write_png("output.png");

}
