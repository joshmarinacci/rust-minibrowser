extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of, call};
use pom::char_class::alpha;
use std::collections::{HashMap, HashSet};
use std::str::{self};

use std::fs::File;
use std::io::Read;

// https://limpet.net/mbrubeck/2014/09/08/toy-layout-engine-5-boxes.html

#[derive(Debug, PartialEq)]
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Text(String),
    Element(ElementData),
}

#[derive(Debug, PartialEq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attributes.get("id")
    }
    pub fn classes(&self) -> HashSet<&str> {
        match self.attributes.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new()
        }
    }
}

type AttrMap = HashMap<String, String>;

fn text(data:String) -> Node {
    Node { children: Vec::new(), node_type:NodeType::Text(data)}
}


fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}
fn v2s(v:&Vec<u8>) -> String {
    str::from_utf8(v).unwrap().to_string()
}

fn attribute<'a>() -> Parser<'a, u8, (String,String)> {
    let p
        = space()
        + is_a(alpha).repeat(1..)
        - sym(b'=')
        - sym(b'"')
        + is_a(alpha).repeat(1..)
        - sym(b'"');
    p.map(|((_, key), value)| {
        return (v2s(&key), v2s(&value))
    })
}

#[test]
fn test_single_attribute() {
    let input = b"foo=\"bar\"";
    println!("{:#?}", attribute().parse(input));
}

fn attributes<'a>() -> Parser<'a, u8, AttrMap> {
    let p = attribute().repeat(0..);
    p.map(|a|{
        let mut map = AttrMap::new();
        for (key,value) in a {
            map.insert(key,value);
        }
        return map;
    })
}

#[test]
fn test_several_attributes() {
    let input = b"foo=\"bar\" baz=\"quxx\" ";
    println!("{:#?}", attributes().parse(input));
}

fn open_element<'a>() -> Parser<'a, u8, (String, AttrMap)> {
    let p
        = space()
        + sym(b'<')
        + is_a(alpha).repeat(1..)
        + attributes()
        - sym(b'>');
    p.map(|((_,name),atts)| {
        (v2s(&name), atts)
    })
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

    p.map(|(((tag_name, attributes), children), _end_name)|{
        Node {
            children,
            node_type: NodeType::Element(ElementData{
                tag_name,
                attributes,
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
fn test_elem_with_attrs() {
    let input = br#"
     <html lang="en">
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


#[test]
fn test_file_load() {
    let mut file = File::open("tests/foo.html").unwrap();
    let mut content: Vec<u8> = Vec::new();
    file.read_to_end(&mut content);
    let parsed = element().parse(content.as_slice()).unwrap();
    println!("{:#?}", parsed);
    let dom = Node {
        node_type: NodeType::Element(ElementData{
            tag_name: "html".to_string(),
            attributes: HashMap::new()
        }),
        children: vec![
            Node {
                node_type: NodeType::Element(ElementData{
                    tag_name: "head".to_string(),
                    attributes: Default::default()
                }),
                children: vec![
                    Node {
                        node_type: NodeType::Element(ElementData{
                            tag_name: "title".to_string(),
                            attributes: Default::default()
                        }),
                        children: vec![text("Title".to_string())]
                    },
                ]
            },
            Node {
                node_type: NodeType::Element(ElementData{
                    tag_name: "body".to_string(),
                    attributes: Default::default()
                }),
                children: vec![text("some text\n".to_string())
                ],
            }
        ]
    };
    assert_eq!(dom,parsed)
}


pub fn load_doc(filename:&str) -> Node {
    let mut file = File::open(filename).unwrap();
    let mut content: Vec<u8> = Vec::new();
    file.read_to_end(&mut content);
    let parsed = element().parse(content.as_slice()).unwrap();
    return parsed;
}

