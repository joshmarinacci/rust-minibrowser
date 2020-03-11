use crate::dom::{load_doc_from_buffer, getElementsByTagName, NodeType, Document, load_doc};
use crate::css::{parse_stylesheet, Stylesheet, parse_stylesheet_from_buffer};
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
    UrlError(),
}
impl From<std::io::Error> for BrowserError {
    fn from(err: Error) -> Self {
        return BrowserError::DiskError(err);
    }
}
impl From<ParseError> for BrowserError {
    fn from(_: ParseError) -> Self {
        unimplemented!()
    }
}
impl From<reqwest::Error> for BrowserError {
    fn from(err: reqwest::Error) -> Self {
        BrowserError::NetworkError(err)
    }
}
impl From<ImageError> for BrowserError {
    fn from(_: ImageError) -> Self {
        unimplemented!()
    }
}
impl From<pom::Error> for BrowserError {
    fn from(_: pom::Error) -> Self {
        unimplemented!()
    }
}

pub fn calculate_url_from_doc(doc:&Document, href:&str) -> Result<Url,BrowserError>{
    println!("going to load {} {}", href, doc.base_url);
    let url = doc.base_url.join(href)?;
    println!("going to the new url {}",url);
    return Ok(url);
}
pub fn file_url_to_path(url:&Url) -> &str {
    url.path()
}

pub fn load_stylesheets_with_fallback(doc:&Document) -> Result<Stylesheet,BrowserError> {
    let style_node = getElementsByTagName(&doc.root_node, "style");
    let default_stylesheet = load_stylesheet_from_net(&relative_filepath_to_url("tests/default.css")?).unwrap();
    println!("loading {:#?}", getElementsByTagName(&doc.root_node,"link"));
    let linked = getElementsByTagName(&doc.root_node, "link");

    if linked.is_some() {
        match &linked.unwrap().node_type {
            Element(ed) => {
                let rel = ed.attributes.get("rel");
                let href = ed.attributes.get("href");
                if rel.is_some() && rel.unwrap() == "stylesheet" && href.is_some() {
                    let href = href.unwrap();
                    let more_ss = load_stylesheet_from_net(&calculate_url_from_doc(doc,href)?);
                    println!("more ss is {:#?}",more_ss);
                }
            }
            _ => {}
        }
    }

    match style_node {
        Some(node) => {
            if let NodeType::Text(text) = &node.children[0].node_type {
                let mut ss = parse_stylesheet(text)?;
                ss.parent = Some(Box::new(default_stylesheet));
                return Ok(ss);
            }
        }
        _ => {}
    }
    return Ok(default_stylesheet);
}

pub fn relative_filepath_to_url(path:&str) -> Result<Url,BrowserError> {
    let cwd = current_dir()?;
    let p = PathBuf::from(path);
    let final_path = cwd.join(p);;
    // println!("final path is {}", final_path.display());
    let base_url = Url::from_file_path(final_path).unwrap();
    // println!("final base url is {}",base_url);
    return Ok(base_url);
}

pub fn url_from_relative_filepath(filepath:&str) -> Result<Url,BrowserError> {
    let foo = Url::parse(&*format!("file://{}", filepath))?;
    return Ok(foo);
}

pub fn load_doc_from_net(url:&Url) -> Result<Document,BrowserError> {
    println!("loading url {}",url);
    match url.scheme() {
        "file" => {
            let path = url.to_file_path().unwrap();
            return load_doc(path.as_path());
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str()).unwrap();
            let status = resp.status();
            let len = resp.content_length();
            println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf);

            let mut doc = load_doc_from_buffer(buf);
            doc.base_url = url.clone();
            return Ok(doc);

        }
    }
}

pub fn load_image_from_net(url:&Url) -> Result<LoadedImage, BrowserError> {
    let mut resp = reqwest::blocking::get(url.as_str())?;
    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf);
    return Ok(load_image_from_buffer(buf)?);
}

pub fn load_stylesheet_from_net(url:&Url) -> Result<Stylesheet, BrowserError>{
    match url.scheme() {
        "file" => {
            let path = url.to_file_path().unwrap();
            let mut file = File::open(path).unwrap();
            let mut content:Vec<u8>= Vec::new();
            file.read_to_end(&mut content);
            let mut ss = parse_stylesheet_from_buffer(content)?;
            ss.base_url = url.clone();
            return Ok(ss);
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str())?;
            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf)?;
            let mut ss = parse_stylesheet_from_buffer(buf)?;
            ss.base_url = url.clone();
            return Ok(ss);
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
            Ok(load_image_from_filepath(file_url_to_path(&url))?)
        },
        _ => {
            Ok(load_image_from_net(&url)?)
        },
    }
}


