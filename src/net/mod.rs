use crate::dom::{load_doc_from_buffer, getElementsByTagName, NodeType, Document, load_doc};
use crate::css::parse_stylesheet;
use crate::style::style_tree;
use crate::image::{load_image_from_buffer, LoadedImage};
use image::ImageError;
use reqwest::Url;
use std::path::PathBuf;

pub fn load_doc_from_net(url:&Url) -> Result<Document,String> {
    let str = String::from(url.as_str());
    println!("loading url {}",url);
    match url.scheme() {
        "file" => {
            println!("this is a file url {}", url.path());
            return load_doc(PathBuf::from(url.path()).as_path());
        }
        _ => {
            let mut resp = reqwest::blocking::get(url.as_str()).unwrap();
            let status = resp.status();
            let len = resp.content_length();
            println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

            let mut buf: Vec<u8> = vec![];
            resp.copy_to(&mut buf);

            let mut doc = load_doc_from_buffer(buf);
            doc.base_url = str;
            return Ok(doc);

        }
    }
}

pub fn load_image_from_net(url:&str) -> Result<LoadedImage, ImageError> {
    let mut resp = reqwest::blocking::get(url).unwrap();
    let status = resp.status();
    let len = resp.content_length();
    println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf);
    return load_image_from_buffer(buf);
}

#[test]
fn test_request() -> Result<(), Box<dyn std::error::Error>> {
    let mut resp = reqwest::blocking::get("https://apps.josh.earth/rust-minibrowser/test1.html")?;
    let status = resp.status();
    let len = resp.content_length();
    println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf)?;
    // println!("text is {:#?}",buf);
    // println!("text is {:#?}",String::from_utf8(buf).unwrap());

    let doc = load_doc_from_buffer(buf);
    // println!("document is {:#?}",doc);
    let res = getElementsByTagName(&doc.root_node, "style");
    println!("result is {:#?}",res);
    if res.is_some() {
        let mut style_node = res.unwrap();
        if let NodeType::Text(text) = &style_node.children[0].node_type {
            println!("got the text {}", text);
            let stylesheet = parse_stylesheet(text);
            println!("got the stylesheet {:#?}",stylesheet);
            let styled = style_tree(&doc.root_node,&stylesheet);
            println!("styled is {:#?}",styled);
        }
    }

    Ok(())
}

