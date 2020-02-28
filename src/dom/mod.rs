extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of,seq, call, not_a};
use pom::char_class::alpha;
use std::collections::HashMap;
use std::str::{self};

use serde_json;
use serde_json::{Value};
use std::fs;


#[derive(Debug, PartialEq)]
struct Node {
    node_type: NodeType,
    children: Vec<Node>,
}

#[derive(Debug, PartialEq)]
enum NodeType {
    Text(String),
    Element(ElementData),
}

#[derive(Debug, PartialEq)]
struct ElementData {
    tag_name: String,
    attributes: AttrMap,
}

type AttrMap = HashMap<String, String>;

fn text(data:String) -> Node {
    Node { children: Vec::new(), node_type:NodeType::Text(data)}
}
fn elem(tag_name:String, attributes:AttrMap, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData {
            tag_name,
            attributes
        })
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}
fn v2s(v:&Vec<u8>) -> String {
    str::from_utf8(v).unwrap().to_string()
}

fn open_element<'a>() -> Parser<'a, u8, String> {
    let p
        = space()
        - sym(b'<')
        + is_a(alpha).repeat(1..)
        - sym(b'>');
    p.map(|(_,name)| v2s(&name))
}
fn close_element<'a>() -> Parser<'a, u8, String> {
    let p
        = space()
        - sym(b'<')
        - sym(b'/')
        + is_a(alpha).repeat(1..)
        - sym(b'>');
    p.map(|(_,name)| v2s(&name))
}
fn text_content<'a>() -> Parser<'a, u8, Node> {
    none_of(b"<").repeat(1..).map(|content|Node{
        children: vec![],
        node_type: NodeType::Text(v2s(&content))
    })
}
fn element_child<'a>() -> Parser<'a, u8, Node> {
    text_content() | element()
}
fn element<'a>() -> Parser<'a, u8, Node> {
    let p
        = open_element()
        - space()
        + call(element_child).repeat(0..)
        - space()
        + close_element();

    p.map(|((name, b), end_name)|{
        Node {
            children: b,//children,
            // children: vec![],
            node_type: NodeType::Element(ElementData{
                tag_name: name,
                attributes: HashMap::new()
            })
        }
    })
}

#[test]
fn test_element() {
    let input = b"<head>";
    println!("{:#?}", open_element().parse(input));
    let input = b"</head>";
    println!("{:#?}", close_element().parse(input));
    let input = b" some foo text ";
    println!("{:#?}", element_child().parse(input));
    let input = b"<head></head>";
    println!("{:#?}", element_child().parse(input));
}

#[test]
fn test_element_text() {
    let input = b"<head> foo </head>";
    println!("{:#?}", element_child().parse(input));
}
#[test]
fn test_element_text_element() {
    let input = b"<head><body></body></head>";
    println!("{:#?}", element().parse(input));
}
#[test]
fn test_nested() {
    let input = br#"
     <html>
       <body>
        <div>some text</div>
       </body>
     </html>
    "#;
    println!("{:#?}", element().parse(input));
}
#[test]
fn test_multi_children() {
    let input = br#"
     <html>
       <body>
        <div>part 1</div>
        <div>part 2</div>
       </body>
     </html>
    "#;
    println!("{:#?}", element().parse(input));
}






pub enum Elem {
    Block(BlockElem),
    Text(TextElem)
}
pub struct BlockElem {
    pub etype:String,
    pub children: Vec<Elem>,
}

pub struct TextElem {
    pub text:String,
}


fn parse_block(json:&Value) -> Elem {
    let rtype = json["type"].as_str().unwrap();
    if rtype == "body" || rtype == "div" {
        println!("parsed {}",rtype);
        let mut block = BlockElem {
            children: Vec::new(),
            etype:rtype.to_string(),
        };
        for child in json["children"].as_array().unwrap() {
            block.children.push(parse_block(&child));
        }
        return Elem::Block(block);
    }

    if rtype == "text" {
        return Elem::Text(TextElem {
            text:json["text"].as_str().unwrap().to_string()
        });
    }

    panic!("found an element type we cant handle")
}

pub fn load_doc(filename:&str) -> Elem {
    let data = fs::read_to_string(filename).expect("file shoudl open");
    let parsed:Value = serde_json::from_str(&data).unwrap();
    println!("parsed the type {}",parsed["type"]);

    return parse_block(&parsed);
}


