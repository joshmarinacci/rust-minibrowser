use crate::dom::{load_doc_from_buffer, getElementsByTagName, NodeType, Document, load_doc};
use crate::css::{parse_stylesheet, Stylesheet, parse_stylesheet_from_buffer, RuleType, Value, parse_stylesheet_from_bytestring};
use crate::style::{dom_tree_to_stylednodes, expand_styles};
use crate::image::{load_image_from_buffer, LoadedImage, load_image_from_filepath};
use image::ImageError;
use std::path::PathBuf;
use std::env::current_dir;
use std::io::{Error, Read};
use url::{Url, ParseError};
use std::fs::File;
use glium_glyph::glyph_brush::rusttype::{Font};
use crate::dom::NodeType::Element;
use glium_glyph::glyph_brush;
use crate::render::FontCache;

#[derive(Debug)]
pub enum BrowserError {
    NetworkError(reqwest::Error),
    DiskError(std::io::Error),
    UrlError(ParseError),
    ImageError(ImageError),
}
impl From<std::io::Error> for BrowserError {
    fn from(err: Error) -> Self {
        BrowserError::DiskError(err)
    }
}
impl From<ParseError> for BrowserError {
    fn from(err: ParseError) -> Self {
        BrowserError::UrlError(err)
    }
}
impl From<reqwest::Error> for BrowserError {
    fn from(err: reqwest::Error) -> Self {
        BrowserError::NetworkError(err)
    }
}
impl From<ImageError> for BrowserError {
    fn from(err: ImageError) -> Self { BrowserError::ImageError(err) }
}
impl From<pom::Error> for BrowserError {
    fn from(_: pom::Error) -> Self {
        unimplemented!()
    }
}
impl From<()> for BrowserError {
    fn from(_: ()) -> Self { unimplemented!()  }
}
impl From<glium_glyph::glyph_brush::rusttype::Error> for BrowserError {
    fn from(_err:glium_glyph::glyph_brush::rusttype::Error) -> Self { unimplemented!()  }
}


pub fn calculate_url_from_doc(doc:&Document, href:&str) -> Result<Url,BrowserError>{
    Ok(doc.base_url.join(href)?)
}

#[derive(Debug)]
pub struct StylesheetSet {
    pub stylesheets:Vec<Stylesheet>,
}

impl StylesheetSet {
    pub fn new() -> Self {
        StylesheetSet {
            stylesheets: vec![]
        }
    }
    pub fn append(&mut self, stylesheet:Stylesheet) {
        self.stylesheets.push(stylesheet)
    }
    pub fn append_from_bytestring(&mut self, font_cache:&mut FontCache, css_text:&[u8]) -> Result<(),BrowserError> {
        let ss = parse_stylesheet_from_bytestring(css_text)?;
        process_stylesheet(self,font_cache,ss)
    }
}

fn process_stylesheet(set:&mut StylesheetSet, font_cache:&mut FontCache, stylesheet:Stylesheet) -> Result<(), BrowserError> {
    //scan for imports
    for rule in stylesheet.rules.iter() {
        if let RuleType::AtRule(ar) = rule {
            if ar.name == "import" {
                println!("got an import ");
                if let Some(Value::FunCall(fcv)) = &ar.value {
                    println!("the url is {:#?}", fcv.arguments);
                    if let Value::StringLiteral(str) = &fcv.arguments[0] {
                        let url = Url::parse(str).unwrap();
                        println!("parsing the imported stylesheet {:#?}", url);
                        load_stylesheet_2(set, font_cache,&url)?;
                    }
                }
            }
        }
    }
    //expand the styles
    let mut ss = stylesheet;
    expand_styles(&mut ss);
    //scan for font face
    font_cache.scan_for_fontface_rules(&ss);
    set.append(ss);
    Ok(())
}
fn load_stylesheet_2(set:&mut StylesheetSet, font_cache:&mut FontCache, url:&Url) -> Result<(), BrowserError> {
    process_stylesheet(set,font_cache,load_stylesheet_from_net(url)?)
}
fn parse_stylesheet_2_from_text(set:&mut StylesheetSet, font_cache:&mut FontCache, text:&String) -> Result<(),BrowserError> {
    process_stylesheet(set,font_cache,parse_stylesheet(text)?)
}

pub fn load_stylesheets_new(doc:&Document, font_cache:&mut FontCache) -> Result<StylesheetSet, BrowserError> {
    let mut set = StylesheetSet::new();
    //load the default stylesheet
    load_stylesheet_2(&mut set, font_cache, &relative_filepath_to_url("tests/default.css")?)?;
    //scan for link nodes
    let link_nodes = getElementsByTagName(&doc.root_node, "link");
    for link in link_nodes.iter() {
        if let Element(ed) = &link.node_type {
            let rel = ed.attributes.get("rel");
            let href = ed.attributes.get("href");
            if rel.is_some() && rel.unwrap() == "stylesheet" && href.is_some() {
                let href = href.unwrap();
                let url = &calculate_url_from_doc(doc, href)?;
                println!("Loading linked stylesheet {:#?}", url);
                load_stylesheet_2(&mut set, font_cache, url);
            }
        }
    }
    //scan for style nodes
    let style_nodes = getElementsByTagName(&doc.root_node, "style");
    for style in style_nodes.iter() {
        if !style.children.is_empty() {
            if let NodeType::Text(text) = &style.children[0].node_type {
                parse_stylesheet_2_from_text(&mut set, font_cache, text)?;
            }
        }
    }
    Ok(set)
}
pub fn relative_filepath_to_url(path:&str) -> Result<Url,BrowserError> {
    let final_path = current_dir()?.join(PathBuf::from(path));
    let base_url = Url::from_file_path(final_path)?;
    Ok(base_url)
}

pub fn load_doc_from_net(url:&Url) -> Result<Document,BrowserError> {
    println!("loading url {}",url);
    match url.scheme() {
        "file" => {
            let path = url.to_file_path()?;
            load_doc(path.as_path())
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str())?;
            let status = resp.status();
            let len = resp.content_length();
            println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf).ok();

            let mut doc = load_doc_from_buffer(buf);
            doc.base_url = url.clone();
            Ok(doc)
        }
    }
}

pub fn load_image_from_net(url:&Url) -> Result<LoadedImage, BrowserError> {
    let mut resp = reqwest::blocking::get(url.as_str())?;
    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf).ok();
    Ok(load_image_from_buffer(buf)?)
}

pub fn load_stylesheet_from_net(url:&Url) -> Result<Stylesheet, BrowserError>{
    // println!("loading stylesheet from url {:#?}",url);
    match url.scheme() {
        "file" => {
            let path = url.to_file_path()?;
            let mut file = File::open(path)?;
            let mut content:Vec<u8>= Vec::new();
            file.read_to_end(&mut content).ok();
            let mut ss = parse_stylesheet_from_buffer(content)?;
            ss.base_url = url.clone();
            Ok(ss)
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str())?;
            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf)?;
            let mut ss = parse_stylesheet_from_buffer(buf)?;
            ss.base_url = url.clone();
            Ok(ss)
        }
    }
}

pub fn load_font_from_net(url:Url) -> Result<Font<'static>, BrowserError> {
    match url.scheme() {
        "file" => {
            let path = url.to_file_path()?;
            let mut file = File::open(path)?;
            let mut content:Vec<u8>= Vec::new();
            file.read_to_end(&mut content).ok();
            Ok(Font::from_bytes(content).unwrap())
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str())?;
            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf)?;
            Ok(Font::from_bytes(buf).unwrap())
        }
    }
}

#[test]
fn test_request() -> Result<(), BrowserError> {
    let mut resp = reqwest::blocking::get("https://apps.josh.earth/rust-minibrowser/test1.html")?;
    let status = resp.status();
    let len = resp.content_length();
    println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf)?;
    let doc = load_doc_from_buffer(buf);
    // println!("document is {:#?}",doc);
    let res = getElementsByTagName(&doc.root_node, "style");
    println!("result is {:#?}",res);
    if !res.is_empty() {
        let style_node = res[0];
        if let NodeType::Text(text) = &style_node.children[0].node_type {
            println!("got the text {}", text);
            let stylesheet = parse_stylesheet(text)?;
            println!("got the stylesheet {:#?}",stylesheet);
            // let styled = dom_tree_to_stylednodes(&doc.root_node, &stylesheet);
            // println!("styled is {:#?}",styled);
        }
    }

    Ok(())
}


pub fn load_image(doc:&Document, href:&str) -> Result<LoadedImage, BrowserError>{
    let url = doc.base_url.join(href)?;
    match url.scheme() {
        "file" => {
            Ok(load_image_from_filepath(url.path().to_string())?)
        },
        _ => {
            Ok(load_image_from_net(&url.clone())?)
        },
    }
}


