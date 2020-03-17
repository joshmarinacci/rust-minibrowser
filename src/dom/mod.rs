extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of, call};
use pom::char_class::alpha;
use std::collections::{HashMap, HashSet};
use std::str::{self};

use std::fs::File;
use std::io::Read;
use self::pom::char_class::alphanum;
use self::pom::parser::{seq, take};
use std::path::Path;
use url::Url;
use crate::net::{BrowserError, load_doc_from_net};
use crate::css::parse_stylesheet;
use std::fmt::Debug;
use self::pom::Error;

// https://limpet.net/mbrubeck/2014/09/08/toy-layout-engine-5-boxes.html

#[derive(Debug, PartialEq)]
pub struct Document {
    pub root_node: Node,
    pub base_url:Url,
}

#[allow(non_snake_case)]
pub fn getElementsByTagName<'a>(node:&'a Node, name:&str) -> Option<&'a Node> {
    if let NodeType::Element(data) = &node.node_type {
        if data.tag_name == name {
            return Some(node);
        }
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
    Comment(String),
    Cdata(String),
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
    let char_string = none_of(b"\"").repeat(0..).convert(String::from_utf8);
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
    p.map(|(_,key)| (v2s(&key),v2s(&key)))
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
    let mut hm = AttrMap::new();
    hm.insert(String::from("foo"), String::from("bar"));
    hm.insert(String::from("baz"), String::from("quxx"));
    assert_eq!(Ok(hm),attributes().parse(b"foo=\"bar\" baz=\"quxx\" "))
}
#[test]
fn test_empty_attribute_value() {
    let mut hm = AttrMap::new();
    hm.insert(String::from("foo"), String::from("bar"));
    assert_eq!(Ok(Node {
        node_type: NodeType::Element(ElementData{ tag_name: "b".to_string(), attributes: hm }),
        children: vec![]
    }), element().parse(br#"<b foo="bar"></b>"#));

    let mut hm = AttrMap::new();
    hm.insert(String::from("foo"), String::from(""));
    assert_eq!(Ok(Node {
        node_type: NodeType::Element(ElementData{ tag_name: "b".to_string(), attributes: hm }),
        children: vec![]
    }),element().parse(br#"<b foo=""></b>"#));
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
    cdata() | comment() | meta_tag() | text_content() | selfclosed_element() | standalone_element() | element()
}
fn standalone_tag<'a>() -> Parser<'a, u8, String> {
    (seq(b"img")|seq(b"link") | seq(b"input") | seq(b"hr") | seq(b"input"))
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

/// Success when sequence of symbols matches current input.
pub fn iseq<'a, 'b: 'a>(tag: &'b [u8]) -> Parser<'a, u8, &'a [u8]>

{
    Parser::new(move |input: &'a [u8], start: usize| {
        let mut index = 0;
        loop {
            let pos = start + index;
            if index == tag.len() {
                return Ok((tag, pos));
            }
            if let Some(s) = input.get(pos) {
                let ch1 = tag[index].to_ascii_lowercase();
                let ch2 = (*s).to_ascii_lowercase();
                if ch1 != ch2 {
                    return Err(Error::Mismatch {
                        message: format!("seq {:?} expect: {:?}, found: {:?}", tag, tag[index], s),
                        position: pos,
                    });
                }
            } else {
                return Err(Error::Incomplete);
            }
            index += 1;
        }
    })
}

//<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
fn doctype<'a>() -> Parser<'a, u8, ()> {
    (iseq(b"<!DOCTYPE") + none_of(b">").repeat(0..) + sym(b'>')).map(|_| ())
}
fn document<'a>() -> Parser<'a, u8, Document> {
    (space().opt() + doctype().opt() + space() + element()).map(|(_,node)| Document {
        root_node: node,
        base_url: Url::parse("https://www.mozilla.org/").unwrap(),
    })
}

#[test]
fn test_doctype() {
    assert_eq!(Ok(()), doctype().parse(b"<!DOCTYPE html>"));
    assert_eq!(Ok(()), doctype().parse(b"<!doctype html>"));
    assert_eq!(Ok(()), doctype().parse(b"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">"));
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

fn cdata<'a>() -> Parser<'a, u8, Node> {
    let p
        = seq(b"<![CDATA[")
        + (!seq(b"]]>") * take(1)).repeat(0..)
        + seq(b"]]>");
    p.map(|((a,c),b)| {
        let mut s:Vec<u8> = Vec::new();
        for cc in c {
            s.push(cc[0]);
        }
        Node{ node_type: NodeType::Cdata(v2s(&s)), children: vec![] }
    })
}
fn comment<'a>() -> Parser<'a, u8, Node> {
    let p
        = seq(b"<!--")
        + (!seq(b"-->") * take(1)).repeat(0..)
        + seq(b"-->");
    p.map(|((a,c),b)| {
        let mut s:Vec<u8> = Vec::new();
        for cc in c {
            s.push(cc[0]);
        }
        Node{ node_type: NodeType::Comment(v2s(&s)), children: vec![] }
    })
}

#[test]
fn test_comment() {
    assert_eq!(Ok(Node{ node_type: NodeType::Comment(" a cool - comment".to_string()), children: vec![] }),
               comment().parse(br"<!-- a cool - comment-->"))
}

#[test]
fn test_comment_2() {
    assert_eq!(Ok(Node{
        node_type: NodeType::Element(ElementData{ tag_name: "foo".to_string(), attributes: Default::default() }),
        children: vec![
            Node{ node_type: NodeType::Comment(" a cool - comment".to_string()), children: vec![] }
        ]
    }), element().parse(br"<foo><!-- a cool - comment--></foo>"));
    assert_eq!(Ok(Node{
        node_type: NodeType::Element(ElementData{ tag_name: "foo".to_string(), attributes: Default::default() }),
        children: vec![
            Node{
                node_type: NodeType::Comment(String::from(" a cool - comment")),
                children: vec![]
            },
            Node{
                node_type: NodeType::Text(String::from("after")),
                children: vec![]
            }
        ]
    }), element().parse(br"<foo><!-- a cool - comment-->after</foo>"));
    assert_eq!(Ok(Node{
        node_type: NodeType::Element(ElementData{ tag_name: "foo".to_string(), attributes: Default::default() }),
        children: vec![
            Node{
                node_type: NodeType::Text(String::from("before")),
                children: vec![]
            },
            Node{
                node_type: NodeType::Comment(String::from(" a cool - comment")),
                children: vec![]
            },
        ]
    }), element().parse(br"<foo>before<!-- a cool - comment--></foo>"));
}


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
    file.read_to_end(&mut content).ok();
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
    file.read_to_end(&mut content).ok();
    let mut parsed = document().parse(content.as_slice()).unwrap();
    let str = filename.to_str().unwrap();
    let base_url = format!("file://{}",str);
    println!("using base url {}", base_url);
    parsed.base_url = Url::parse(base_url.as_str()).unwrap();
    Ok(parsed)
}
pub fn load_doc_from_buffer(buf:Vec<u8>) -> Document {
    document().parse(buf.as_slice()).unwrap()
}
pub fn load_doc_from_bytestring(buf:&[u8]) -> Document {
    document().parse(buf).unwrap()
}


pub fn strip_empty_nodes(doc:&mut Document) {
    strip_empty_nodes_helper(&mut doc.root_node);
}
fn strip_empty_nodes_helper(node:&mut Node) {
    node.children.retain(|ch| {
        match &ch.node_type {
            NodeType::Text(str) => {
                // println!("got a text node -{}-",str.trim());
                if str.trim().len() == 0 {
                    // println!("empty node. must prune it");
                    false
                } else {
                    true
                }
            }
            _ => true
        }
    });
    for ch in node.children.iter_mut() {
        strip_empty_nodes_helper(ch);
    }
}

#[test]
fn test_strip_empty_nodes() {
    let input = br#"
    <html>
        <body>
            <div>blah</div>
        </body>
    </html>
    "#;
    let mut doc = document().parse(input).unwrap();
    println!("{:?}", doc);

    strip_empty_nodes(&mut doc);
    assert_eq!(
        Document{
            root_node: Node {
                node_type: NodeType::Element(ElementData{
                    tag_name: "html".to_string(),
                    attributes: Default::default()
                }),
                children: vec![
                    Node {
                        node_type: NodeType::Element(ElementData {
                            tag_name:"body".to_string(),
                            attributes: Default::default()
                        }),
                        children: vec![
                            Node {
                                node_type: NodeType::Element(ElementData {
                                    tag_name:"div".to_string(),
                                    attributes: Default::default()
                                }),
                                children: vec![
                                    Node {
                                        node_type: NodeType::Text(String::from("blah")),
                                        children: vec![]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            },
            base_url: Url::parse("https://www.mozilla.org/").unwrap()
        },
        doc
        );
}

pub fn expand_entities(doc:&mut Document) {
    expand_entities_helper(&mut doc.root_node);
}
fn expand_entities_helper(node:&mut Node) {
    for ch in node.children.iter_mut() {
        match &ch.node_type {
            NodeType::Text(str) => {
                let mut str2 = String::from(str);
                str2 = str2.replace("&lt;","<");
                str2 = str2.replace("&gt;",">");
                str2 = str2.replace("&amp;","&");
                ch.node_type = NodeType::Text(str2);
            }
            _ => {}
        }
        expand_entities_helper(ch);
    }
}

#[test]
fn test_expand_entities() {
    let input = br#"
    <html>
        <body>
            <div>&lt; &gt; &amp;</div>
        </body>
    </html>
    "#;
    let mut doc = document().parse(input).unwrap();
    strip_empty_nodes(&mut doc);
    expand_entities(&mut doc);
    println!("{:?}", doc);
    assert_eq!(
        Document{
            root_node: Node {
                node_type: NodeType::Element(ElementData{
                    tag_name: "html".to_string(),
                    attributes: Default::default()
                }),
                children: vec![
                    Node {
                        node_type: NodeType::Element(ElementData {
                            tag_name:"body".to_string(),
                            attributes: Default::default()
                        }),
                        children: vec![
                            Node {
                                node_type: NodeType::Element(ElementData {
                                    tag_name:"div".to_string(),
                                    attributes: Default::default()
                                }),
                                children: vec![
                                    Node {
                                        node_type: NodeType::Text(String::from("< > &")),
                                        children: vec![]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            },
            base_url: Url::parse("https://www.mozilla.org/").unwrap()
        },
        doc
    );

}


