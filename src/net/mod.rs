use crate::dom::{load_doc_from_buffer, getElementsByTagName, NodeType, Document, load_doc};
use crate::css::{parse_stylesheet, Stylesheet, parse_stylesheet_from_buffer, RuleType, Value};
use crate::style::style_tree;
use crate::image::{load_image_from_buffer, LoadedImage, load_image_from_filepath};
use image::ImageError;
use std::path::PathBuf;
use std::env::current_dir;
use std::io::{Error, Read};
use url::{Url, ParseError};
use std::fs::File;
use crate::dom::NodeType::Element;

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

pub fn load_stylesheets_with_fallback(doc:&Document) -> Result<Stylesheet,BrowserError> {
    let style_node = getElementsByTagName(&doc.root_node, "style");
    let ss1 = load_stylesheet_from_net(&relative_filepath_to_url("tests/default.css")?)?;
    println!("loading {:#?}", getElementsByTagName(&doc.root_node,"link"));
    let linked = getElementsByTagName(&doc.root_node, "link");

    if let Some(node) = linked {
        if let Element(ed) = &node.node_type {
            let rel = ed.attributes.get("rel");
            let href = ed.attributes.get("href");
            if rel.is_some() && rel.unwrap() == "stylesheet" && href.is_some() {
                let href = href.unwrap();
                let more_ss = load_stylesheet_from_net(&calculate_url_from_doc(doc,href)?);
                println!("more ss is {:#?}",more_ss);
            }
        }
    }

    if let Some(node) = style_node {
        if !node.children.is_empty() {
            if let NodeType::Text(text) = &node.children[0].node_type {
                let mut ss2 = parse_stylesheet(text)?;
                // println!("parsed inline styles {:#?}", ss2);
                ss2.parent = Some(Box::new(ss1));
                for rule in ss2.rules.iter() {
                    if let RuleType::AtRule(ar) = rule {
                        if ar.name == "import" {
                            println!("got an import ");
                            if let Some(Value::FunCall(fcv)) = &ar.value {
                                // println!("the url is {:#?}", fcv.arguments);
                                if let Value::StringLiteral(str) = &fcv.arguments[0] {
                                    load_stylesheet_from_net(&Url::parse(str).unwrap())?;
                                    println!("parsed the remote stylesheet {:#?}", fcv.arguments);
                                }
                            }
                        }
                    }
                }
                return Ok(ss2);
            }
        }
    }
    Ok(ss1)
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
    if res.is_some() {
        let style_node = res.unwrap();
        if let NodeType::Text(text) = &style_node.children[0].node_type {
            println!("got the text {}", text);
            let stylesheet = parse_stylesheet(text)?;
            println!("got the stylesheet {:#?}",stylesheet);
            let styled = style_tree(&doc.root_node,&stylesheet);
            println!("styled is {:#?}",styled);
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


