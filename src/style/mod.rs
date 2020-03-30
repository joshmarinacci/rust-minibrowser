use crate::dom::{Node, ElementData, NodeType, load_doc_from_bytestring, strip_empty_nodes};
use crate::css::{Selector, SimpleSelector, Rule, Stylesheet, Specificity, Value, Color, parse_stylesheet_from_bytestring, Unit, RuleType, Declaration};
use std::collections::HashMap;
use crate::css::Selector::{Simple, Ancestor};
use crate::dom::NodeType::{Element, Text, Meta};
use crate::css::Value::{Keyword, ColorValue, Length, HexColor,};
use crate::net::{load_stylesheet_from_net, relative_filepath_to_url, load_doc_from_net, load_stylesheets_with_fallback};
use std::fs::File;
use std::io::BufReader;

type PropertyMap = HashMap<String, Value>;


fn load_css_json() -> HashMap<String, Color>{
    println!("loading css-color-names.json");
    let file = File::open("res/css-color-names.json").unwrap();
    let reader = BufReader::new(file);
    let json:serde_json::Value = serde_json::from_reader(reader).unwrap();

    let mut map:HashMap<String,Color> = HashMap::new();
    if let serde_json::Value::Object(obj) = json {
        for (key, value) in obj.iter() {
            if let serde_json::Value::String(val) = value {
                map.insert(key.to_string(),Color::from_hex(&*val));
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
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
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
        self.specified_values.get(name).cloned()
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
    pub fn lookup_string(&self, name:&str, default: &str) -> String {
        match self.value(name) {
            Some(Value::StringLiteral(txt)) => txt,
            Some(Keyword(str)) => str,
            _ => default.to_string(),
        }
    }
    pub fn lookup_keyword(&self, name:&str, default: &Value) -> Value {
        match self.value(name) {
            Some(Value::Keyword(txt)) => Keyword(txt),
            _ => default.clone(),
        }
    }
    pub fn lookup_font_weight(&self, default:i32) -> i32{
        match self.lookup("font-weight", "font-weight",&Keyword(String::from("normal"))) {
            Keyword(str) => match str.as_str() {
                "normal" => 400,
                "bold" => 700,
                "inherit" => {
                    println!("!!!inherited font weight. this should already be taken care of!!!");
                    default
                }
                _ => default,
            },
            Value::Number(v) => v as i32,
            _ => default,
        }
    }
    pub fn lookup_length_px(&self, name:&str, default:f32) -> f32 {
        match self.value(name) {
            Some(Length(v,_unit)) => v,
            _ => default,
        }
    }
    pub fn display(&self) -> Display {
        if let Text(_) = self.node.node_type {
            return Display::Inline
        }
        match self.value("display") {
            Some(Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                "inline-block" => Display::InlineBlock,
                "table" => Display::Table,
                "table-row-group" => Display::TableRowGroup,
                "table-row" => Display::TableRow,
                "table-cell" => Display::TableCell,
                _ => {
                    println!("WARNING: unsupported display keyword {}",s);
                    Display::Inline
                },
            },
            _ => Display::Inline,
        }
    }

    pub fn color(&self, name: &str) -> Option<Color> {
        match self.value(name) {
            Some(ColorValue(c)) => Some(c),
            Some(HexColor(str)) => Some(Color::from_hex(&str)),
            Some(Keyword(name)) => find_color_lazy_static(&name),
            Some(Length(_,_)) => None,
            None => None,
            _ => None,
        }
    }
    pub fn insets(&self, name: &str) -> f32 {
        match self.value(name) {
            Some(Length(v,_unit)) => v,
            _ => 0.0,
        }
    }
}

fn matches(elem: &ElementData, selector: &Selector, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> bool {
    match *selector {
        Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
        Ancestor(ref sel) => {
            println!("ANCESTOR NOT SUPPORTED YET");

            let child_match = matches(elem, &*sel.child, ancestors);
            let mut parent_match = false;
            if !ancestors.is_empty() {
                let (parent_node,_) = &ancestors[0];
                if let NodeType::Element(ed) = &parent_node.node_type {
                    parent_match = matches(ed, &*sel.ancestor, ancestors);
                }
            }
            child_match && parent_match
        }
    }
}


fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    //return false for mis-matches
    if selector.tag_name.iter().any(|name|  "*" != *name)
        && selector.tag_name.iter().any(|name| elem.tag_name != *name) {
            return false;
    }
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }
    let elem_classes = elem.classes();
    if selector.class.iter().any(|class| !elem_classes.contains(&**class)) {
        return false
    }
    //no non-matching selectors found, so it must be true
    true
}

type MatchedRule<'a> = (Specificity, &'a Rule);

// return rule that matches, if any.
fn match_rule<'a>(elem: &ElementData, rule: &'a Rule, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> Option<MatchedRule<'a>> {
    rule.selectors.iter()
        .find(|selector| matches(elem, selector, ancestors))
        .map(|selector| (selector.specificity(), rule))
}

fn only_real_rules(rtype:&RuleType) -> Option<&Rule> {
    match rtype {
        RuleType::Rule(rule) => Some(&rule),
        _ => None,
    }
}
//find all matching rules for an element
fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> Vec<MatchedRule<'a>> {
    let mut rules:Vec<MatchedRule> = match &stylesheet.parent {
        Some(parent) => parent.rules.iter()
            .filter_map(only_real_rules)
            .filter_map(|rule| match_rule(elem, &rule,ancestors)).collect(),
        None => vec![],
    };
    let mut rules2:Vec<MatchedRule> = stylesheet.rules.iter()
        .filter_map(only_real_rules)
        .filter_map(|rule| match_rule(elem,&rule,ancestors)).collect();
    rules.append(&mut rules2);
    rules
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
    let mut rules = matching_rules(elem,stylesheet,ancestors);

    //sort rules by specificity
    rules.sort_by(|&(a,_),&(b,_)| a.cmp(&b));
    for (_,rule) in rules {
        for declaration in &rule.declarations {
            // println!("checking {} {:#?}", declaration.name, declaration.value);
            let vv = calculate_inherited_property_value(declaration, ancestors);
            values.insert(declaration.name.clone(), vv);
        }
    }
    values
}

//returns inherited value if inherit is set and prop name is found, or just returns the original value
fn calculate_inherited_property_value(dec:&Declaration, ancestors:&mut Vec::<(&Node, &PropertyMap)>) -> Value {
    if dec.value == Keyword(String::from("inherit")) {
        for (_node, props) in ancestors.iter() {
            if props.contains_key(&*dec.name) {
                return props.get(&*dec.name).unwrap().clone();
            }
        }
    }
    dec.value.clone()
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    let mut ansc:Vec<(&Node, &PropertyMap)> = vec![];
    real_style_tree(root, stylesheet, &mut ansc)
}
pub fn real_style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet, ancestors:&mut Vec::<(&Node,&PropertyMap)>) -> StyledNode<'a> {
    let specified = match root.node_type {
        Element(ref elem) => specified_values(elem, stylesheet, ancestors),
        Text(_) => HashMap::new(),
        Meta(_) => HashMap::new(),
        _ => HashMap::new(),
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
        <b>cool<a>rad</a></b>
    </html>
    "#;
    let css_text = br#"
        * {
            color: inherit;
            font-weight: inherit;
        }
        html {
            color: black;
            font-weight: bold;
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

    //check html element
    assert_eq!(snode.specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));

    // check html b element
    assert_eq!(snode.children[0].specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));
    assert_eq!(snode.children[0].specified_values.get("font-weight").unwrap(),
               &Keyword(String::from("bold")));
    // check html b a element
    assert_eq!(snode.children[0].children[1].specified_values.get("color").unwrap(),
               &Keyword(String::from("blue")));
    assert_eq!(snode.children[0].children[1].specified_values.get("font-weight").unwrap(),
               &Keyword(String::from("bold")));

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

    //check html element
    assert_eq!(snode.specified_values.get("margin").unwrap(), &Length(1.0,Unit::Em));
}

#[test]
fn test_vertical_align() {
    let doc_text = br#"<html>
    <style type="text/css">
        .top {
            vertical-align: top;
        }
    </style>
    <div class="top">top</div>
    </html>"#;
    let mut doc = load_doc_from_bytestring(doc_text);
    strip_empty_nodes(&mut doc);
    let stylesheet = load_stylesheets_with_fallback(&doc).unwrap();
    let snode = style_tree(&doc.root_node, &stylesheet);
    // println!("doc={:#?} stylesheet={:#?} snode={:#?}",doc,stylesheet,snode);
    let div = &snode.children[1];
    let text = &div.children[0];
    println!("specified values are {:#?}",text.value("color"));
    assert_eq!(div.lookup_string("vertical-align","foo"),"top".to_string());
}

#[test]
fn test_style_tree() {
    let doc = load_doc_from_net(&relative_filepath_to_url("tests/test1.html").unwrap()).unwrap();
    let stylesheet = load_stylesheet_from_net(&relative_filepath_to_url("tests/foo.css").unwrap()).unwrap();
    let snode = style_tree(&doc.root_node,&stylesheet);
    println!("final snode is {:#?}",snode)
}


#[test]
fn test_multi_selector_match() {
    let doc_text = br#"
    <html>
        <b>cool</b><a>rad</a>
    </html>
    "#;
    let css_text = br#"
        * {
            color: black;
        }
        a,b {
            color:red;
        }
    "#;
    let doc = load_doc_from_bytestring(doc_text);
    let stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    let snode = style_tree(&doc.root_node, &stylesheet);
    println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);

    //check html element
    assert_eq!(snode.specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));

    // check html b element
    assert_eq!(snode.children[0].specified_values.get("color").unwrap(),
               &Keyword(String::from("red")));
    // check html a element
    assert_eq!(snode.children[1].specified_values.get("color").unwrap(),
               &Keyword(String::from("red")));

}

#[test]
fn test_ancestor_match() {
    let doc_text = br#"
    <b><a>rad</a></b>
    "#;
    let css_text = br#"
        * {
            color: black;
        }
        b a {
            color:red;
        }
    "#;
    let doc = load_doc_from_bytestring(doc_text);
    let stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    let snode = style_tree(&doc.root_node, &stylesheet);
    println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);

    //check b
    assert_eq!(snode.specified_values.get("color").unwrap(),
               &Keyword(String::from("black")));

    // check b a
    assert_eq!(snode.children[0].specified_values.get("color").unwrap(),
               &Keyword(String::from("red")));

}
fn expand_array_decl(new_decs:&mut Vec::<Declaration>, dec:&Declaration) {
    match &dec.value {
        Value::ArrayValue(arr) => {
            if arr.len() == 2 {
                new_decs.push(Declaration {
                    name: format!("{}-top",dec.name),
                    value: arr[0].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-right",dec.name),
                    value: arr[1].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-bottom",dec.name),
                    value: arr[0].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-left",dec.name),
                    value: arr[1].clone()
                });
            }
            if arr.len() == 4 {
                new_decs.push(Declaration {
                    name: format!("{}-top",dec.name),
                    value: arr[0].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-right",dec.name),
                    value: arr[1].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-bottom",dec.name),
                    value: arr[2].clone()
                });
                new_decs.push(Declaration {
                    name: format!("{}-left",dec.name),
                    value: arr[3].clone()
                });
            }
        }
        Value::Length(_, _) => {
            new_decs.push(Declaration {
                name: format!("{}-top",dec.name),
                value: dec.value.clone()
            });
            new_decs.push(Declaration {
                name: format!("{}-right",dec.name),
                value: dec.value.clone()
            });
            new_decs.push(Declaration {
                name: format!("{}-bottom",dec.name),
                value: dec.value.clone()
            });
            new_decs.push(Declaration {
                name: format!("{}-left",dec.name),
                value: dec.value.clone()
            });
        }
        _ => {
            new_decs.push(dec.clone());
        }
    }
}

pub fn expand_styles(ss:&mut Stylesheet) {
    for rule in ss.rules.iter_mut() {
        if let RuleType::Rule(rule) = rule {
            let mut new_decs = vec![];
            for dec in rule.declarations.iter_mut() {
                // println!("decl = {:#?}",dec);
                match dec.name.as_str() {
                    "margin" => expand_array_decl(&mut new_decs, dec),
                    "padding" => expand_array_decl(&mut new_decs, dec),
                    "border-width" => expand_array_decl(&mut new_decs, dec),
                    "border" => expand_border_shorthand(&mut new_decs, dec),
                    _ => new_decs.push(dec.clone()),
                }
            }
            rule.declarations = new_decs;
        }
    }
}

fn expand_border_shorthand(new_decs:&mut Vec::<Declaration>, dec:&Declaration) {
    // println!("expanding border shorthand: {:#?}",dec);
    match &dec.value {
        Value::ArrayValue(vec) => {
            if vec.len() != 3 {
                panic!("border shorthand must have three values");
            }
            new_decs.push(Declaration{
                name: String::from("border-width-top"),
                value: vec[0].clone()
            });
            new_decs.push(Declaration{
                name: String::from("border-width-left"),
                value: vec[0].clone()
            });
            new_decs.push(Declaration{
                name: String::from("border-width-right"),
                value: vec[0].clone()
            });
            new_decs.push(Declaration{
                name: String::from("border-width-bottom"),
                value: vec[0].clone()
            });
            new_decs.push(Declaration{
                name: String::from("border-style"),
                value: vec[1].clone()
            });
            new_decs.push(Declaration{
                name: String::from("border-color"),
                value: vec[2].clone()
            });
        }
        _ => {
            panic!("border shorthand must be an array value");
        }
    }
}

#[test]
fn test_property_expansion_1() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            margin: 1px;
            border-width: 1px;
        }
    "#;

    let doc = load_doc_from_bytestring(doc_text);
    let mut stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    expand_styles(&mut stylesheet);
    let mut snode = style_tree(&doc.root_node, &stylesheet);
    println!("stylesheet is {:#?}",stylesheet);
    assert_eq!(snode.lookup_length_px("margin-top",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-right",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-bottom",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-left",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-top",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-right",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-bottom",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-left",5.0),1.0);
}

#[test]
fn test_property_expansion_2() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            margin: 1px 2px;
        }
    "#;

    let doc = load_doc_from_bytestring(doc_text);
    let mut stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    expand_styles(&mut stylesheet);
    let mut snode = style_tree(&doc.root_node, &stylesheet);
    println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);
    assert_eq!(snode.lookup_length_px("margin-top",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-right",5.0),2.0);
    assert_eq!(snode.lookup_length_px("margin-bottom",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-left",5.0),2.0);
}


#[test]
fn test_property_expansion_4() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            margin: 1px 2px 3px 4px;
        }
    "#;

    let doc = load_doc_from_bytestring(doc_text);
    let mut stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    expand_styles(&mut stylesheet);
    let mut snode = style_tree(&doc.root_node, &stylesheet);
    println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);
    assert_eq!(snode.lookup_length_px("margin-top",5.0),1.0);
    assert_eq!(snode.lookup_length_px("margin-right",5.0),2.0);
    assert_eq!(snode.lookup_length_px("margin-bottom",5.0),3.0);
    assert_eq!(snode.lookup_length_px("margin-left",5.0),4.0);
}

#[test]
fn test_border_shorthand() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            border: 1px solid black;
        }
    "#;

    let doc = load_doc_from_bytestring(doc_text);
    let mut stylesheet = parse_stylesheet_from_bytestring(css_text).unwrap();
    expand_styles(&mut stylesheet);
    let mut snode = style_tree(&doc.root_node, &stylesheet);
    println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);
    assert_eq!(snode.lookup_length_px("border-width-top",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-right",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-bottom",5.0),1.0);
    assert_eq!(snode.lookup_length_px("border-width-left",5.0),1.0);
    assert_eq!(snode.lookup_keyword("border-color", &Keyword(String::from("white"))), Keyword(String::from("black")));
}
