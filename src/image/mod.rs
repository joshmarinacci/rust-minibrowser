extern crate image;
use std::fmt::{Formatter};
use std::fmt;
use self::image::{ImageError, RgbaImage};
use self::image::io::Reader;
use std::io::Cursor;

pub struct LoadedImage {
    pub path:String,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub image2d: RgbaImage,
}

impl fmt::Display for LoadedImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "foo")
    }
}
impl fmt::Debug for LoadedImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "foo")
    }
}

fn img_to_loaded_image(img:RgbaImage, path:String) -> Result<LoadedImage, ImageError> {
    let (w,h) = img.dimensions();
    let loaded = LoadedImage {
        path: path,
        width: w as i32,
        height: h as i32,
        image2d: img,
    };
    Result::Ok(loaded)
}
pub fn load_image_from_filepath(path:String) -> Result<LoadedImage, ImageError> {
    let img = image::open(path.clone())?.into_rgba();
    img_to_loaded_image(img, path)
}

pub fn load_image_from_buffer(buf:Vec<u8>) -> Result<LoadedImage, ImageError>{
    let reader = Reader::new(Cursor::new(buf)).with_guessed_format().expect("cursor io never fails");
    let img = reader.decode()?;
    img_to_loaded_image(img.into_rgba(),"none".to_string())
}
