extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of, call};
use pom::char_class::alpha;
use std::collections::{HashMap, HashSet};
use std::str::{self};

use std::fs::File;
use std::io::Read;
use self::pom::char_class::alphanum;
use self::pom::parser::{seq, take};
use crate::css::parse_stylesheet;
use std::path::Path;
use url::Url;
use crate::net::{BrowserError, load_doc_from_net};

// https://limpet.net/mbrubeck/2014/09/08/toy-layout-engine-5-boxes.html

#[derive(Debug, PartialEq)]
pub struct Document {
    pub root_node: Node,
    pub base_url:Url,
}

#[allow(non_snake_case)]
pub fn getElementsByTagName<'a>(node:&'a Node, name:&str) -> Option<&'a Node> {
    match &node.node_type {
        NodeType::Element(data) => {
            if data.tag_name == name {
                return Some(node);
            }
        },
        _  => {},
    }

    for child in node.children.iter() {
        let res = getElementsByTagName(&child, name);
        if res.is_some() { return res }
    }

    None
}

#[derive(Debug, PartialEq)]
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Text(String),
    Element(ElementData),
    Meta(MetaData),
}

#[derive(Debug, PartialEq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

#[derive(Debug, PartialEq)]
pub struct MetaData {
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

fn alphanum_string<'a>() -> Parser<'a, u8, String> {
    let r = is_a(alphanum).repeat(1..);
    r.map(|str| String::from_utf8(str).unwrap())
}

fn element_name<'a>() -> Parser<'a,u8,String> {
    alphanum_string()
}
#[test]
fn test_element_name() {
    let input = br#"div"#;
    let result = element_name().parse(input);
    println!("{:?}", result);
    assert_eq!(String::from("div"), result.unwrap());
}

#[test]
fn test_element_name_with_number() {
    let input = br#"h3"#;
    let result = element_name().parse(input);
    println!("{:?}", result);
    assert_eq!(String::from("h3"), result.unwrap());
}

fn attribute<'a>() -> Parser<'a, u8, (String,String)> {
    let char_string = none_of(b"\\\"").repeat(1..).convert(String::from_utf8);
    let p
        = space()
        + is_a(alpha).repeat(1..)
        - sym(b'=')
        - space()
        - sym(b'"')
        + char_string
        - sym(b'"');
    p.map(|((_,key),value)| (v2s(&key), value))
}
fn standalone_attribute<'a>() -> Parser<'a, u8, (String,String)> {
    let p
        = space()
        + is_a(alpha).repeat(1..)
        ;
    p.map(|(_,key)| (v2s(&key).clone(),v2s(&key).clone()))
}

#[test]
fn test_attribute_simple() {
    let input = b"foo=\"bar\"";
    println!("{:#?}", attribute().parse(input));
}
#[test]
fn test_attribute_complex() {
    let input = b"foo=\"bar-foo-8\"";
    println!("{:#?}", attribute().parse(input));
}
#[test]
fn test_attribute_standalone() {
    let input = b"foo=\"bar\" baz";
    println!("{:#?}", attributes().parse(input));
}
fn attributes<'a>() -> Parser<'a, u8, AttrMap> {
    let p = (attribute() | standalone_attribute()).repeat(0..);
    p.map(|a|{
        let mut map = AttrMap::new();
        for (key,value) in a {
            map.insert(key,value);
        }
        map
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
        + alphanum_string()
        + attributes()
        - space()
        - sym(b'>');
    p.map(|((_,name),atts)| {
        (name, atts)
    })
}
fn close_element<'a>() -> Parser<'a, u8, String> {
    let p
        = space()
        - sym(b'<')
        - sym(b'/')
        + alphanum_string()
        - sym(b'>');
    p.map(|(_,name)| name)
}
fn text_content<'a>() -> Parser<'a, u8, Node> {
    none_of(b"<").repeat(1..).map(|content|Node{
        children: vec![],
        node_type: NodeType::Text(v2s(&content))
    })
}
fn element_child<'a>() -> Parser<'a, u8, Node> {
    meta_tag() | text_content() | selfclosed_element() | standalone_element() | element()
}
fn standalone_tag<'a>() -> Parser<'a, u8, String> {
    (seq(b"img")|seq(b"link") | seq(b"input"))
        .map(|f| v2s(&f.to_vec()))
}

fn selfclosed_element<'a>() -> Parser<'a, u8, Node> {
    let p
        = space()
        + sym(b'<')
        + standalone_tag()
        + attributes()
        - space()
        - seq(b"/>");
    p.map(|((_, tag_name), attributes)|{
        Node {
            node_type: NodeType::Element(ElementData{
                tag_name,
                attributes,
            }),
            children: vec![],
        }
    })
}

fn standalone_element<'a>() -> Parser<'a, u8, Node> {
    let p
        = space()
        + sym(b'<')
        + standalone_tag()
        + attributes()
        - sym(b'>');
    p.map(|((_, tag_name), attributes)|{
        Node {
            node_type: NodeType::Element(ElementData{
                tag_name,
                attributes,
            }),
            children: vec![],
        }
    })
}

#[test]
fn test_standlone_elements() {
    assert!(standalone_element().parse(b"<img>").is_ok());
    assert!(standalone_element().parse(br#"<img src="foo.png">"#).is_ok());
    assert!(standalone_element().parse(b"<link>").is_ok());
    assert!(element_child().parse(b"<link/>").is_ok());
}

#[test]
fn test_standalone_attribute() {
    assert!(element_child().parse(br#"<iframe width="853" height="480" src="https://www.youtube.com/embed/YslQ2625TR4" frameborder="0" allowfullscreen></iframe>"#).is_ok());

}

fn element<'a>() -> Parser<'a, u8, Node> {
    let p
        = open_element()
        - space()
//        - comment()
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
    assert!(open_element().parse(b"<head>").is_ok());
    assert!(close_element().parse(b"</head>").is_ok());
    assert!(element_child().parse(b"some foo text").is_ok());
    assert!(element_child().parse(b"<head></head>").is_ok());
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
fn test_multi_children_h3() {
    let input = br#"
     <html>
       <body>
        <div>part 1</div>
        <h3>part 2</h3>
       </body>
     </html>
    "#;
    println!("{:#?}", element().parse(input));
}

#[test]
fn test_div_with_img_child() {
    let input = br#"
     <div>
        <img src="foo.png">
     </div>
    "#;
    println!("{:#?}", element().parse(input));
}

fn doctype<'a>() -> Parser<'a, u8, ()> {
     seq(b"<!DOCTYPE html>").map(|_| ())
}
fn document<'a>() -> Parser<'a, u8, Document> {
    (space().opt() + doctype().opt() + space() + element()).map(|(_,node)| Document {
        root_node: node,
        base_url: Url::parse("https://www.mozilla.org/").unwrap(),
    })
}

#[test]
fn test_doctype() {
    let input = br#"<!DOCTYPE html>"#;
    let result = doctype().parse(input);
    println!("{:?}", result);
    assert_eq!((), result.unwrap());
}


fn meta_tag<'a>() -> Parser<'a, u8, Node> {
    let p = seq(b"<meta ") + attributes() - (seq(b">") | seq(b"/>"));
    p.map(|(_,attributes)| Node {
        node_type: NodeType::Meta(MetaData{
            attributes,
        }),
        children: vec![]
    })
}

#[test]
fn test_metatag() {
    let input = br#"<meta charset="UTF-8">"#;
    let result = meta_tag().parse(input);
    println!("{:?}", result);
    let mut atts = HashMap::new();
    atts.insert("charset".to_string(),"UTF-8".to_string());
    assert_eq!(Node{
        node_type: NodeType::Meta(MetaData {
            attributes: atts
        }),
        children: vec![]
    }, result.unwrap());
}

#[test]
fn test_metatag_with_closing_element() {
    assert!(meta_tag().parse(b"<meta />").is_ok())
}

#[test]
fn test_linktag_with_closing_element() {
    assert!(element_child().parse(br#"<link rel="stylesheet" href="tufte.css"/>"#).is_ok())
}
#[test]
fn test_input_element() {
    assert!(element_child().parse(br#"<input />"#).is_ok());
    assert!(element_child().parse(br#"<input ></input>"#).is_ok());
}

fn comment<'a>() -> Parser<'a, u8, ()> {
    let p = seq(b"<!--") + (!seq(b"-->") + take(1)).repeat(0..) + seq(b"-->");
    p.map(|((_,_),b)| {
        println!("comment {}",v2s(&b.to_vec()));
    })
}
/*
#[test]
fn test_comment() {
    let input = br"<!-- a cool - comment-->";
    let result = comment().parse(input);
    println!("{:?}", result);
    assert_eq!((),result.unwrap())
}

#[test]
fn test_comment_2() {
    let input = br"<foo> and a better <!-- a cool - comment--></foo>";
    let result = document().parse(input);
    println!("{:?}", result);
    // assert_eq!((),result.unwrap())
}
*/

#[test]
fn test_style_parse() {
    let input = br#"<head>
    <style type="text/css">
      .foo {
        color:red;
       }
    </style>
    </head>"#;
    let result = document().parse(input);
    println!("{:?}", result);
    match &result.unwrap().root_node.children[0].children[0].node_type {
        NodeType::Text(txt) => {
            println!("got the text {}",txt);
            let ss = parse_stylesheet(txt);
            println!("stylesheet is {:#?}",ss);
        },
        _ => {}
    }
}

#[test]
fn test_simple_doc() {
    let input = br#"
    <!DOCTYPE html>
<html>
    <head>
        <meta charset="UTF-8"></head></html>
    "#;
    let result = document().parse(input);
    println!("foo");
    println!("{:?}", result);
    let mut atts = HashMap::new();
    atts.insert("charset".to_string(),"UTF-8".to_string());
    assert_eq!(Document{
        root_node: Node {
            node_type: NodeType::Element(ElementData{
                tag_name: "html".to_string(),
                attributes: Default::default()
            }),
            children: vec![
                Node {
                    node_type: NodeType::Element(ElementData {
                        tag_name:"head".to_string(),
                        attributes: Default::default()
                    }),
                    children: vec![
                        Node{
                            node_type: NodeType::Meta(MetaData{ attributes: atts }),
                            children: vec![]
                        }
                    ]
                }
            ]
        },
        base_url: Url::parse("https://www.mozilla.org/").unwrap()
    }, result.unwrap());
}

#[test]
fn test_file_load() {
    let mut file = File::open("tests/foo.html").unwrap();
    let mut content: Vec<u8> = Vec::new();
    file.read_to_end(&mut content);
    let parsed = document().parse(content.as_slice()).unwrap();
    println!("{:#?}", parsed);
    let dom = Document {
        root_node: Node {
            node_type: NodeType::Element(ElementData {
                tag_name: "html".to_string(),
                attributes: HashMap::new()
            }),
            children: vec![
                Node {
                    node_type: NodeType::Element(ElementData {
                        tag_name: "head".to_string(),
                        attributes: Default::default()
                    }),
                    children: vec![
                        Node {
                            node_type: NodeType::Element(ElementData {
                                tag_name: "title".to_string(),
                                attributes: Default::default()
                            }),
                            children: vec![text("Title".to_string())]
                        },
                    ]
                },
                Node {
                    node_type: NodeType::Element(ElementData {
                        tag_name: "body".to_string(),
                        attributes: Default::default()
                    }),
                    children: vec![text("some text".to_string())
                    ],
                }
            ]
        },
        base_url: Url::parse("https://www.mozilla.org/").unwrap()
    };
    assert_eq!(dom,parsed)
}

#[test]
fn test_tufte() {
    let mut file = File::open("tests/tufte/tufte.html").unwrap();
    let mut input: Vec<u8> = Vec::new();
    file.read_to_end(&mut input);
    let mut result = document().parse(input.as_slice());
    //
    // println!("error is {:#?}",result.err());
    // for (i,bt) in input.iter().enumerate() {
    //     println!("foo {} {} {} {}", i, (*bt) as char, bt, 47 as char);
    // }
    assert!(result.is_ok())
}

pub fn load_doc(filename:&Path) -> Result<Document,BrowserError> {
    println!("Loading doc from file {}", filename.display());
    let mut file = File::open(filename).unwrap();
    let mut content: Vec<u8> = Vec::new();
    file.read_to_end(&mut content);
    let mut parsed = document().parse(content.as_slice()).unwrap();
    let str = filename.to_str().unwrap();
    let base_url = format!("file://{}",str);
    println!("using base url {}", base_url);
    parsed.base_url = Url::parse(base_url.as_str()).unwrap();
    return Ok(parsed);
}
pub fn load_doc_from_buffer(buf:Vec<u8>) -> Document {
    document().parse(buf.as_slice()).unwrap()
}
pub fn load_doc_from_bytestring(buf:&[u8]) -> Document {
    document().parse(buf).unwrap()
}

