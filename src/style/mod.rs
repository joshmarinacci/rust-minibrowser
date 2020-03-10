use crate::dom::{Node, ElementData, load_doc, Document, NodeType, load_doc_from_buffer, load_doc_from_bytestring};
use crate::css::{Selector, SimpleSelector, Rule, Stylesheet, Specificity, Value, Color, parse_stylesheet_from_bytestring, Unit};
use std::collections::HashMap;
use crate::css::Selector::Simple;
use crate::dom::NodeType::{Element, Text, Meta};
use crate::css::Value::{Keyword, ColorValue, Length, HexColor,};
use crate::render::{BLACK, BLUE, RED, GREEN, WHITE, AQUA, YELLOW};
use crate::net::{load_stylesheet_from_net, relative_filepath_to_url, load_doc_from_net};
use std::fs::File;
use std::io::Read;
use std::io::BufReader;

type PropertyMap = HashMap<String, Value>;


fn hexstring_to_color(str:&str) -> Color {
    let n = i32::from_str_radix(&str[1..], 16).unwrap();
    let r = (n >> 16) & 0xFF;
    let g = (n >> 8) & 0xFF;
    let b = (n >> 0) & 0xFF;
    Color {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: 255
    }
}
fn load_css_json() -> HashMap<String, Color>{
    println!("loading css-color-names.json");
    let file = File::open("res/css-color-names.json").unwrap();
    let reader = BufReader::new(file);
    let json:serde_json::Value = serde_json::from_reader(reader).unwrap();

    let mut map:HashMap<String,Color> = HashMap::new();
    if let serde_json::Value::Object(obj) = json {
        for (key, value) in obj.iter() {
            if let serde_json::Value::String(val) = value {
                map.insert(key.to_string(),hexstring_to_color(&*val));
            }
        }
    }
    map
}

lazy_static! {
    pub static ref COLORS_MAP: HashMap<String, Color> = { load_css_json() };
}
pub fn find_color_lazy_static(name: &str) -> Option<Color> {
    COLORS_MAP.get(&name.to_lowercase()).cloned()
}

#[derive(Debug)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    None,
}

#[derive(Debug, PartialEq)]
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl StyledNode<'_> {
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).map(|v| v.clone())
    }
    pub fn lookup(&self, name:&str, fallback_name: &str, default: &Value) -> Value {
        self.value(name).unwrap_or_else(||self.value(fallback_name)
            .unwrap_or_else(||default.clone()))
    }
    pub fn lookup_color(&self, name:&str, default: &Color) -> Color {
        match self.color(name) {
            Some(color) => color,
            _ => default.clone(),
        }
    }
    pub fn lookup_length_px(&self, name:&str, default:f32) -> f32 {
        match self.value(name) {
            Some(Length(v,_unit)) => {
                return v;
            },
            _ => default,
        }
    }
    pub fn display(&self) -> Display {
        match self.node.node_type {
            Text(_) => {
                return Display::Inline;
            }
            _ => {}
        }
        match self.value("display") {
            Some(Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                "inline-block" => Display::InlineBlock,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }

    pub fn color(&self, name: &str) -> Option<Color> {
        match self.value(name) {
            Some(ColorValue(c)) => Some(c),
            Some(HexColor(str)) => {
                let n = i32::from_str_radix(&str[1..],16).unwrap();
                let r = (n >> 16) & 0xFF;
                let g = (n >>  8) & 0xFF;
                let b = (n >>  0) & 0xFF;
                Some(Color{
                    r: r as u8,
                    g: g as u8,
                    b: b as u8,
                    a: 255
                })
            },
            Some(Keyword(name)) => find_color_lazy_static(&name),
            Some(Length(_,_)) => None,
            None => None,
        }
    }
    pub fn insets(&self, name: &str) -> f32 {
        match self.value(name) {
            Some(Length(v,_unit)) => v,
            _ => 0.0,
        }
    }
}

fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match *selector {
        Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector)
    }
}


fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    //return false for mis-matches
    if selector.tag_name.iter().any(|name|  "*" != *name) {
        if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
            return false;
        }
    }
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }
    let elem_classes = elem.classes();
    if selector.class.iter().any(|class| !elem_classes.contains(&**class)) {
        return false
    }
    //no non-matching selectors found, so it must be true
    return true;
}

type MatchedRule<'a> = (Specificity, &'a Rule);

// return rule that matches, if any.
fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    rule.selectors.iter()
        .find(|selector| matches(elem, selector))
        .map(|selector| (selector.specificity(), rule))
}

//find all matching rules for an element
fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    let mut rules:Vec<MatchedRule> = match &stylesheet.parent {
        Some(parent) => parent.rules.iter().filter_map(|rule| match_rule(elem,rule)).collect(),
        None => vec![],
    };
    let mut rules2:Vec<MatchedRule> = stylesheet.rules.iter().filter_map(|rule| match_rule(elem,rule)).collect();
    rules.append(&mut rules2);
    return rules;
}

#[test]
fn test_multifile_cascade() {
    let stylesheet_parent = load_stylesheet_from_net(&relative_filepath_to_url("tests/default.css").unwrap()).unwrap();
    let mut stylesheet = load_stylesheet_from_net(&relative_filepath_to_url("tests/child.css").unwrap()).unwrap();
    stylesheet.parent = Some(Box::new(stylesheet_parent));
    let elem = ElementData {
        tag_name: String::from("div"),
        attributes: Default::default()
    };
    let mut a2:Vec<(&Node, &PropertyMap)> = vec![];
    let values = specified_values(&elem, &stylesheet, &mut a2);
    println!("got the values {:#?}", values);
    assert_eq!(values.get("background-color").unwrap(),&Value::Keyword(String::from("blue")));
}

// get all values set by all rules
fn specified_values(elem: &ElementData, stylesheet: &Stylesheet, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> PropertyMap {
    // println!("styling with ancestors {:#?}", ancestors.len());
    // for an in ancestors.iter() {
    //     println!("   ancestor {:#?} {:#?}", an.0.node_type, an.1);
    // }
    let mut values:HashMap<String,Value> = HashMap::new();
    let mut rules = matching_rules(elem,stylesheet);

    //sort rules by specificity
    rules.sort_by(|&(a,_),&(b,_)| a.cmp(&b));
    for (_,rule) in rules {
        for declaration in &rule.declarations {
            // println!("checking {} {:#?}", declaration.name, declaration.value);
            let mut vv = &declaration.value;
            if declaration.name == "color" && declaration.value == Keyword(String::from("inherit")) {
                // println!("other inherit");
                for (node,props) in ancestors.iter() {
                    if props.contains_key("color") {
                        // println!("found an ancestor match {:#?}", props.get("color"));
                        vv = props.get("color").unwrap();
                    }
                }
            }
            values.insert(declaration.name.clone(), vv.clone());
        }
    }
    return values;
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    let mut ansc:Vec<(&Node, &PropertyMap)> = vec![];
    return real_style_tree(root, stylesheet, &mut ansc);
}
pub fn real_style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> StyledNode<'a> {
    let specified = match root.node_type {
        Element(ref elem) => specified_values(elem, stylesheet, ancestors),
        Text(_) => HashMap::new(),
        Meta(_) => HashMap::new(),
    };
    let mut a2:Vec<(&Node, &PropertyMap)> = vec![];
    a2.push((root, &specified));
    let ch2 = root.children.iter().map(|child| real_style_tree(child, stylesheet, &mut a2)).collect();
    StyledNode {
        node: root,
        specified_values: specified,
        children: ch2,
    }
}

#[test]
fn test_inherited_match() {
    let doc_text = br#"
    <html>
        <b>cool</b>
    </html>
    "#;
    let css_text = br#"
        * {
            color: inherit;
        }
        html {
            color: black;
        }
        b {
            foo:bar;
        }
        a {
            color: blue;
        }
    "#;
    let doc = load_doc_from_bytestring(doc_text);
    let stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    let snode = style_tree(&doc.root_node, &stylesheet);
    //println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);
    dump_stylednode(&snode);

    //check html element
    assert_eq!(snode.specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));

    // check html b element
    assert_eq!(snode.children[0].specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));

    // check html b text element
    // assert_eq!(snode.children[0].children[0].specified_values.get("color").unwrap(),
    //            &Keyword(String::from("black")));
    // println!("done")

}

#[test]
fn test_em_to_px() {
    let doc_text = br#" <html> <p>cool</p> </html> "#;
    let css_text = br#"
        * {
            color: inherit;
        }
        html {
            color: black;
            margin: 1em;
        }
        p {
            color: black;
            margin: 1em;
        }
    "#;
    let doc = load_doc_from_bytestring(doc_text);
    let stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    let snode = style_tree(&doc.root_node, &stylesheet);
    // println!("doc={:#?} stylesheet={:#?} snode={:#?}",doc,stylesheet,snode);
    // dump_stylednode(&snode);

    //check html element
    assert_eq!(snode.specified_values.get("margin").unwrap(), &Length(1.0,Unit::Em));
}

fn dump_stylednode(node:&StyledNode) {
    println!("node {}", dump_nodetype(&node.node.node_type));
    println!(" styles {:#?}", node.specified_values);
    for ch in node.children.iter() {
        dump_stylednode(&ch);
    }
}
fn dump_nodetype(typ:&NodeType) -> String {
    match typ {
        Text(txt) => format!("TEXT -{}-",txt),
        Element(ed) => format!("{}",&ed.tag_name),
        _ => {"other".to_string()}
    }
}

#[test]
fn test_style_tree() {
    let doc = load_doc_from_net(&relative_filepath_to_url("tests/test1.html").unwrap()).unwrap();
    let stylesheet = load_stylesheet_from_net(&relative_filepath_to_url("tests/foo.css").unwrap()).unwrap();
    let snode = style_tree(&doc.root_node,&stylesheet);
    println!("final snode is {:#?}",snode)
}

