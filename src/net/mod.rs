use crate::dom::{load_doc_from_buffer, getElementsByTagName, NodeType, Document};
use crate::css::parse_stylesheet;
use crate::style::style_tree;

pub fn load_doc_from_net(url:&str) -> Option<Document> {
    let mut resp = reqwest::blocking::get("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    let status = resp.status();
    let len = resp.content_length();
    println!("{:#?}\n content length = {:#?}\n status = {:#?}", resp, len, status);

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf);
    // println!("text is {:#?}",buf);
    // println!("text is {:#?}",String::from_utf8(buf).unwrap());

    let doc = load_doc_from_buffer(buf);
    return Some(doc);
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
