use crate::css::Selector::{Ancestor, Simple};
use crate::css::Value::{ColorValue, HexColor, Keyword, Length};
use crate::css::{
    Color, Declaration, Rule, RuleType, Selector, SimpleSelector, Specificity, Stylesheet, Unit,
    Value,
};
use crate::dom::NodeType::{Element, Meta, Text};
use crate::dom::{ElementData, Node, NodeType};
use crate::net::StylesheetSet;
use crate::render::FontCache;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::rc::{Rc, Weak};

type PropertyMap = HashMap<String, Value>;

fn load_css_json() -> HashMap<String, Color> {
    println!("loading css-color-names.json");
    let file = File::open("res/css-color-names.json").unwrap();
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).unwrap();

    let mut map: HashMap<String, Color> = HashMap::new();
    if let serde_json::Value::Object(obj) = json {
        for (key, value) in obj.iter() {
            if let serde_json::Value::String(val) = value {
                map.insert(key.to_string(), Color::from_hex(&*val));
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
    ListItem,
    None,
}

#[derive(Debug)]
pub struct StyledNode {
    pub node: Node,
    pub children: RefCell<Vec<Rc<StyledNode>>>,
    parent: RefCell<Weak<StyledNode>>,
    pub specified_values: PropertyMap,
}

#[derive(Debug)]
pub struct StyledTree {
    pub root: RefCell<Rc<StyledNode>>,
}
impl StyledTree {
    pub fn new() -> Self {
        StyledTree {
            root: RefCell::new(Rc::new(StyledNode {
                node: Node {
                    node_type: NodeType::Comment(String::from("comment")),
                    children: vec![],
                },
                children: RefCell::new(vec![]),
                parent: RefCell::new(Default::default()),
                specified_values: Default::default(),
            })),
        }
    }
    pub fn make(&self) -> Rc<StyledNode> {
        Rc::new(StyledNode {
            node: Node {
                node_type: NodeType::Comment(String::from("comment")),
                children: vec![],
            },
            children: RefCell::new(vec![]),
            parent: RefCell::new(Weak::new()),
            specified_values: Default::default(),
        })
    }
    pub fn make_with(
        &self,
        node: Node,
        specified_values: PropertyMap,
        children: RefCell<Vec<Rc<StyledNode>>>,
    ) -> Rc<StyledNode> {
        let rc = Rc::new(StyledNode {
            node,
            children,
            parent: RefCell::new(Default::default()),
            specified_values,
        });
        for ch in rc.children.borrow().iter() {
            *ch.parent.borrow_mut() = Rc::downgrade(&rc);
        }
        rc
    }
    pub fn set_root(&self, node: Rc<StyledNode>) {
        *self.root.borrow_mut() = node;
    }
    pub fn append(&self, parent: &Rc<StyledNode>, child: &Rc<StyledNode>) {
        parent.children.borrow_mut().push(Rc::clone(child));
        *child.parent.borrow_mut() = Rc::downgrade(parent);
    }
}

impl StyledNode {
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }
    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }
    pub fn lookup_color(&self, name: &str, default: &Color) -> Color {
        match self.color(name) {
            Some(color) => color,
            _ => default.clone(),
        }
    }
    pub fn lookup_string(&self, name: &str, default: &str) -> String {
        match self.value(name) {
            Some(Value::StringLiteral(txt)) => txt,
            Some(Keyword(str)) => str,
            _ => default.to_string(),
        }
    }
    pub fn lookup_keyword(&self, name: &str, default: &Value) -> Value {
        match self.value(name) {
            Some(Value::Keyword(txt)) => Keyword(txt),
            _ => default.clone(),
        }
    }
    pub fn lookup_text_decoration_line(&self) -> String {
        let val = self.lookup_keyword(
            "text-decoration-line",
            &Value::Keyword(String::from("none")),
        );
        if let Keyword(str) = val {
            str
        } else {
            "none".to_string()
        }
    }
    pub fn lookup_font_weight(&self, default: i32) -> i32 {
        match self.lookup(
            "font-weight",
            "font-weight",
            &Keyword(String::from("normal")),
        ) {
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
    pub fn lookup_font_family(&self, font_cache: &mut FontCache) -> String {
        let font_family_values = self.lookup(
            "font-family",
            "font-family",
            &Value::Keyword(String::from("sans-serif")),
        );
        // println!("font family values: {:#?} {:#?}",font_family_values, self);
        match font_family_values {
            Value::ArrayValue(vals) => {
                for val in vals.iter() {
                    match val {
                        Value::StringLiteral(str) => {
                            if font_cache.has_font_family(str) {
                                return String::from(str);
                            }
                        }
                        Value::Keyword(str) => {
                            if font_cache.has_font_family(str) {
                                return String::from(str);
                            }
                        }
                        _ => {}
                    }
                }
                println!("no valid font found in stack: {:#?}", vals);
                String::from("sans-serif")
            }
            Value::Keyword(str) => str,
            _ => String::from("sans-serif"),
        }
    }

    pub fn lookup_length_px(&self, name: &str, default: f32) -> f32 {
        match self.value(name) {
            Some(Length(v, _unit)) => v,
            _ => default,
        }
    }
    pub fn lookup_font_size(&self) -> f32 {
        match self.value("font-size") {
            Some(Length(v, unit)) => {
                match unit {
                    Unit::Px => v,
                    Unit::Per => {
                        v / 100.0 * self.parent.borrow().upgrade().unwrap().lookup_font_size()
                    }
                    Unit::Em => v * self.parent.borrow().upgrade().unwrap().lookup_font_size(),
                    Unit::Rem => v * 18.0, //TODO: use the real document font-size for REMs
                }
            }
            _ => {
                println!("unrecognized font-size type {:#?}", self.value("font-size"));
                10.0
            }
        }
    }

    pub fn lookup_length_as_px(&self, name: &str, default: f32) -> f32 {
        if let Some(value) = self.value(name) {
            match value {
                Length(v, Unit::Px) => v,
                Length(v, Unit::Em) => v * self.lookup_font_size(),
                Length(v, Unit::Rem) => v * self.lookup_font_size(),
                // TODO: use real document font size
                Length(_v, Unit::Per) => {
                    println!("WARNING: percentage in length_to_px. should have be converted to pixels already");
                    default
                }
                _ => default,
            }
        } else {
            default
        }
    }

    pub fn display(&self) -> Display {
        if let Text(_) = self.node.node_type {
            return Display::Inline;
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
                "list-item" => Display::ListItem,
                _ => {
                    println!("WARNING: unsupported display keyword {}", s);
                    Display::Inline
                }
            },
            _ => Display::Inline,
        }
    }

    pub fn color(&self, name: &str) -> Option<Color> {
        match self.value(name) {
            Some(ColorValue(c)) => Some(c),
            Some(HexColor(str)) => Some(Color::from_hex(&str)),
            Some(Keyword(name)) => find_color_lazy_static(&name),
            Some(Length(_, _)) => None,
            None => None,
            _ => None,
        }
    }
    pub fn insets(&self, name: &str) -> f32 {
        match self.value(name) {
            Some(Length(v, _unit)) => v,
            _ => 0.0,
        }
    }
}

fn matches(
    elem: &ElementData,
    selector: &Selector,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> bool {
    match *selector {
        Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
        Ancestor(ref sel) => {
            println!("ANCESTOR NOT SUPPORTED YET");

            let child_match = matches(elem, &*sel.child, ancestors);
            let mut parent_match = false;
            if !ancestors.is_empty() {
                let (parent_node, _) = &ancestors[0];
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
    if selector.tag_name.iter().any(|name| "*" != *name)
        && selector.tag_name.iter().any(|name| elem.tag_name != *name)
    {
        return false;
    }
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }
    let elem_classes = elem.classes();
    if selector
        .class
        .iter()
        .any(|class| !elem_classes.contains(&**class))
    {
        return false;
    }
    //no non-matching selectors found, so it must be true
    true
}

type MatchedRule<'a> = (Specificity, &'a Rule);

// return rule that matches, if any.
fn match_rule<'a>(
    elem: &ElementData,
    rule: &'a Rule,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> Option<MatchedRule<'a>> {
    rule.selectors
        .iter()
        .find(|selector| matches(elem, selector, ancestors))
        .map(|selector| (selector.specificity(), rule))
}

fn only_real_rules(rtype: &RuleType) -> Option<&Rule> {
    match rtype {
        RuleType::Rule(rule) => Some(&rule),
        _ => None,
    }
}
//find all matching rules for an element
fn matching_rules<'a>(
    elem: &ElementData,
    styles: &'a StylesheetSet,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> Vec<MatchedRule<'a>> {
    let mut rules2: Vec<MatchedRule> = vec![];
    for sheet in styles.stylesheets.iter() {
        let mut rules: Vec<MatchedRule> = sheet
            .rules
            .iter()
            .filter_map(only_real_rules)
            .filter_map(|rule| match_rule(elem, &rule, ancestors))
            .collect();
        rules2.append(&mut rules);
    }
    rules2
}

// get all values set by all rules
fn specified_values(
    elem: &ElementData,
    styles: &StylesheetSet,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> PropertyMap {
    // println!("styling with ancestors {:#?}", ancestors.len());
    // for an in ancestors.iter() {
    //     println!("   ancestor {:#?} {:#?}", an.0.node_type, an.1);
    // }
    let mut values: HashMap<String, Value> = HashMap::new();
    let mut rules = matching_rules(elem, styles, ancestors);

    //sort rules by specificity
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            // println!("checking {} {:#?}", declaration.name, declaration.value);
            let vv = calculate_inherited_property_value(declaration, ancestors);
            values.insert(declaration.name.clone(), vv);
        }
    }
    values
}

//returns inherited value if inherit is set and prop name is found, or just returns the original value
fn calculate_inherited_property_value(
    dec: &Declaration,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> Value {
    if dec.value == Keyword(String::from("inherit")) {
        for (_node, props) in ancestors.iter() {
            if props.contains_key(&*dec.name) {
                let newval = props.get(&*dec.name).unwrap();
                if newval == &Keyword(String::from("inherit")) {
                    continue;
                }
                return newval.clone();
            }
        }
    }
    dec.value.clone()
}

pub fn dom_tree_to_stylednodes<'a>(root: &'a Node, styles: &'a StylesheetSet) -> StyledTree {
    let tree = StyledTree::new();
    let mut ansc: Vec<(&Node, &PropertyMap)> = vec![];
    tree.set_root(real_style_tree(&tree, root, styles, &mut ansc));
    tree
}

fn real_style_tree<'a>(
    tree: &StyledTree,
    root: &'a Node,
    styles: &'a StylesheetSet,
    ancestors: &mut Vec<(&Node, &PropertyMap)>,
) -> Rc<StyledNode> {
    let specified = match root.node_type {
        Element(ref elem) => specified_values(elem, styles, ancestors),
        Text(_) => HashMap::new(),
        Meta(_) => HashMap::new(),
        _ => HashMap::new(),
    };
    let mut a2: Vec<(&Node, &PropertyMap)> = vec![];
    a2.push((root, &specified));
    let ch2: Vec<Rc<StyledNode>> = root
        .children
        .iter()
        .map(|child| real_style_tree(tree, child, styles, &mut a2))
        .collect();
    tree.make_with((*root).clone(), specified, RefCell::new(ch2))
}

fn expand_array_decl(new_decs: &mut Vec<Declaration>, dec: &Declaration) {
    match &dec.value {
        Value::ArrayValue(arr) => {
            if arr.len() == 2 {
                new_decs.push(Declaration {
                    name: format!("{}-top", dec.name),
                    value: arr[0].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-right", dec.name),
                    value: arr[1].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-bottom", dec.name),
                    value: arr[0].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-left", dec.name),
                    value: arr[1].clone(),
                });
            }
            if arr.len() == 4 {
                new_decs.push(Declaration {
                    name: format!("{}-top", dec.name),
                    value: arr[0].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-right", dec.name),
                    value: arr[1].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-bottom", dec.name),
                    value: arr[2].clone(),
                });
                new_decs.push(Declaration {
                    name: format!("{}-left", dec.name),
                    value: arr[3].clone(),
                });
            }
        }
        Value::Length(_, _) => {
            new_decs.push(Declaration {
                name: format!("{}-top", dec.name),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: format!("{}-right", dec.name),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: format!("{}-bottom", dec.name),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: format!("{}-left", dec.name),
                value: dec.value.clone(),
            });
        }
        _ => {
            new_decs.push(dec.clone());
        }
    }
}

pub fn expand_styles(ss: &mut Stylesheet) {
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

fn expand_border_shorthand(new_decs: &mut Vec<Declaration>, dec: &Declaration) {
    // println!("expanding border shorthand: {:#?}",dec);
    match &dec.value {
        Value::ArrayValue(vec) => {
            if vec.len() != 3 {
                panic!("border shorthand must have three values");
            }
            new_decs.push(Declaration {
                name: String::from("border-width-top"),
                value: vec[0].clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-left"),
                value: vec[0].clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-right"),
                value: vec[0].clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-bottom"),
                value: vec[0].clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-style"),
                value: vec[1].clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-color"),
                value: vec[2].clone(),
            });
        }
        Value::Number(_) => {
            new_decs.push(Declaration {
                name: String::from("border-width-top"),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-left"),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-right"),
                value: dec.value.clone(),
            });
            new_decs.push(Declaration {
                name: String::from("border-width-bottom"),
                value: dec.value.clone(),
            });
        }
        _ => {
            panic!("border shorthand must be an array value {:#?}", dec);
        }
    }
}

#[test]
fn test_multifile_cascade() {
    let stylesheet_parent =
        load_stylesheet_from_net(&relative_filepath_to_url("tests/default.css").unwrap()).unwrap();
    let mut stylesheet =
        load_stylesheet_from_net(&relative_filepath_to_url("tests/child.css").unwrap()).unwrap();
    let elem = ElementData {
        tag_name: String::from("div"),
        attributes: Default::default(),
    };
    let mut a2: Vec<(&Node, &PropertyMap)> = vec![];
    let mut styles = StylesheetSet::new();
    styles.append(stylesheet_parent);
    styles.append(stylesheet);
    let values = specified_values(&elem, &styles, &mut a2);
    println!("got the values {:#?}", values);
    assert_eq!(
        values.get("background-color").unwrap(),
        &Value::Keyword(String::from("blue"))
    );
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
    let (doc, sss, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    //println!("doc is {:#?} {:#?} {:#?}",doc,stylesheet,snode);

    //check html element
    assert_eq!(
        snode.specified_values.get("color").unwrap(),
        &Keyword(String::from("black"))
    );

    // check html b element
    assert_eq!(
        snode.children.borrow()[0]
            .specified_values
            .get("color")
            .unwrap(),
        &Keyword(String::from("black"))
    );
    assert_eq!(
        snode.children.borrow()[0]
            .specified_values
            .get("font-weight")
            .unwrap(),
        &Keyword(String::from("bold"))
    );
    // check html b a element
    assert_eq!(
        snode.children.borrow()[0].children.borrow()[1]
            .specified_values
            .get("color")
            .unwrap(),
        &Keyword(String::from("blue"))
    );
    assert_eq!(
        snode.children.borrow()[0].children.borrow()[1]
            .specified_values
            .get("font-weight")
            .unwrap(),
        &Keyword(String::from("bold"))
    );
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
    let (doc, sss, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc={:#?}  snode={:#?}", doc, snode);

    //check html element
    assert_eq!(
        snode.specified_values.get("margin-left").unwrap(),
        &Length(1.0, Unit::Em)
    );
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
    let (doc, sss, stree, lbox, rbox) = standard_test_run(doc_text, br"").unwrap();
    let root = &stree.root.borrow();
    let div = &root.children.borrow()[1];
    // println!("the styles are {:#?}", sss);
    assert_eq!(
        div.lookup_string("vertical-align", "foo"),
        "top".to_string()
    );
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
    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, snode);

    //check html element
    assert_eq!(
        snode.specified_values.get("color").unwrap(),
        &Keyword(String::from("black"))
    );

    // check html b element
    assert_eq!(
        snode.children.borrow()[0]
            .specified_values
            .get("color")
            .unwrap(),
        &Keyword(String::from("red"))
    );
    // check html a element
    assert_eq!(
        snode.children.borrow()[1]
            .specified_values
            .get("color")
            .unwrap(),
        &Keyword(String::from("red"))
    );
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
    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, snode);

    //check b
    assert_eq!(
        snode.specified_values.get("color").unwrap(),
        &Keyword(String::from("black"))
    );

    // check b a
    let f = snode.children.borrow();
    let f2 = &f[0];
    f2.specified_values.get("color").unwrap();
    assert_eq!(
        snode.children.borrow()[0]
            .specified_values
            .get("color")
            .unwrap(),
        &Keyword(String::from("red"))
    );
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

    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("stylesheet is {:#?}", stylesheet);
    assert_eq!(snode.lookup_length_px("margin-top", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-right", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-bottom", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-left", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-top", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-right", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-bottom", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-left", 5.0), 1.0);
}

#[test]
fn test_property_expansion_2() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            margin: 1px 2px;
        }
    "#;

    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, snode);
    assert_eq!(snode.lookup_length_px("margin-top", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-right", 5.0), 2.0);
    assert_eq!(snode.lookup_length_px("margin-bottom", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-left", 5.0), 2.0);
}

#[test]
fn test_property_expansion_4() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            margin: 1px 2px 3px 4px;
        }
    "#;

    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, snode);
    assert_eq!(snode.lookup_length_px("margin-top", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("margin-right", 5.0), 2.0);
    assert_eq!(snode.lookup_length_px("margin-bottom", 5.0), 3.0);
    assert_eq!(snode.lookup_length_px("margin-left", 5.0), 4.0);
}

#[test]
fn test_border_shorthand() {
    let doc_text = br#"<div></div>"#;
    let css_text = br#"
        div {
            border: 1px solid black;
        }
    "#;

    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    let snode = stree.root.borrow();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, snode);
    assert_eq!(snode.lookup_length_px("border-width-top", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-right", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-bottom", 5.0), 1.0);
    assert_eq!(snode.lookup_length_px("border-width-left", 5.0), 1.0);
    assert_eq!(
        snode.lookup_keyword("border-color", &Keyword(String::from("white"))),
        Keyword(String::from("black"))
    );
}

#[test]
fn test_relative_font_sizes() {
    let doc_text = br#"<body><p>stuff</p></body>"#;
    let css_text = br#"body { font-size: 10px; } p { font-size: 200%; }"#;
    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(doc_text, css_text).unwrap();
    println!("doc is {:#?} {:#?} {:#?}", doc, stylesheet, stree);
    let style_root = stree.root.borrow();
    assert_eq!(style_root.lookup_font_size(), 10.0);
    let style_child = &style_root.children.borrow()[0];
    assert_eq!(style_child.lookup_font_size(), 20.0);
}

#[test]
fn test_default_styles() {
    let (doc, stylesheet, stree, lbox, rbox) = standard_test_run(
        br#"<html><body><div>foo</div></body></html>"#,
        br#"html { color: red; }"#,
    )
    .unwrap();

    let html_style = stree.root.borrow();
    assert_eq!(
        html_style.lookup_color("color", &Color::from_hex("#000000")),
        Color::from_hex("#ff0000")
    );
    assert_eq!(
        html_style.lookup_keyword("display", &Value::Keyword(String::from("foo"))),
        Value::Keyword("block".to_string())
    );
}
