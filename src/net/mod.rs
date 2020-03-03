use crate::dom::{load_doc_from_buffer, getElementsByTagName};

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

    Ok(())
}
