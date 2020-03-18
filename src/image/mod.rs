extern crate image;
use image::{GenericImageView};
use raqote::{Image, DrawTarget,  DrawOptions};
use std::fmt::{Debug, Formatter};
use std::fmt;
use self::image::ImageError;
use self::image::io::Reader;
use std::io::Cursor;
use crate::net::load_image_from_net;
use url::Url;

#[derive(Debug,PartialEq)]
pub struct LoadedImage {
    path:String,
    pub(crate) width: i32,
    pub(crate) height: i32,
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

pub fn load_image_from_filepath(path:&str) -> Result<LoadedImage, ImageError> {
    let img = image::open(path)?;
    let (w,h) = img.dimensions();

    let mut loaded = LoadedImage {
        path: path.to_string(),
        width: w as i32,
        height: h as i32,
        data: vec![255 as u32;(w*h) as usize]
    };
    // let mut data2 = vec![255 as u32;(w*h) as usize];
    for (x,y,pixel) in img.pixels() {
        let n = (y*w+x) as usize;
        loaded.data[n]
            = 0xFF_00_00_00
            | ((pixel[0] as u32)<<16)
            | ((pixel[1] as u32)<< 8)
            |  (pixel[2] as u32)
        ;
    }
    Result::Ok(loaded)
}

pub fn load_image_from_buffer(buf:Vec<u8>) -> Result<LoadedImage, ImageError>{
    let reader = Reader::new(Cursor::new(buf)).with_guessed_format().expect("cursor io never fails");
    let img = reader.decode()?;
    // let img = image::open(buf)?;
    let (w,h) = img.dimensions();
    let mut loaded = LoadedImage {
        path: String::from("--network--"),
        width: w as i32,
        height: h as i32,
        data: vec![255 as u32;(w*h) as usize]
    };
    // let mut data2 = vec![255 as u32;(w*h) as usize];
    for (x,y,pixel) in img.pixels() {
        let n = (y*w+x) as usize;
        loaded.data[n]
            = 0xFF_00_00_00
            | ((pixel[0] as u32)<<16)
            | ((pixel[1] as u32)<< 8)
            |  (pixel[2] as u32)
        ;
    }
    Result::Ok(loaded)
}

#[test]
fn test_image_load() {
    let image  = load_image_from_filepath("tests/images/cat.jpg").unwrap();
    let mut dt = DrawTarget::new(image.width, image.height);
    dt.draw_image_at(0.,0., &image.to_image(), &DrawOptions::default());
    dt.write_png("output.png");
}

#[test]
fn test_remote_image_load() {
    // let image  = load_image_from_path("tests/images/cat.jpg").unwrap();
    let image = load_image_from_net(&Url::parse("https://apps.josh.earth/rust-minibrowser/cat.jpg").unwrap()).unwrap();

    let mut dt = DrawTarget::new(image.width, image.height);
    dt.draw_image_at(0.,0., &image.to_image(), &DrawOptions::default());
    dt.write_png("output.png");
}
