extern crate pom;
use pom::parser::*;
use pom::parser::Parser;
use pom::char_class::alpha;
use std::collections::HashMap;
use std::str::{self, FromStr};
use self::pom::char_class::alphanum;
use std::fs::File;
use std::io::Read;
use font_kit::properties::Style;


struct Stylesheet {
    rules: Vec<Rule>,
}

struct Rule {
    selectors: Vec<Selector>,
    declarations: Vec<Declaration>,
}
#[derive(Debug, PartialEq)]
enum Selector {
    Simple(SimpleSelector)
}

#[derive(Debug, PartialEq)]
struct SimpleSelector {
    tag_name: Option<String>,
    id: Option<String>,
    class: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct Declaration {
    name: String,
    value: Value,
}

#[derive(Debug, PartialEq)]
enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug, PartialEq)]
enum Unit {
    Px,
}

#[derive(Debug, PartialEq)]
struct Color {
    r:u8,
    g:u8,
    b:u8,
    a:u8,
}

#[test]
fn make_stylesheet() {
    let ss = Stylesheet {
        rules: vec![
            Rule {
                selectors: vec![
                    Selector::Simple(SimpleSelector{
                        tag_name: Some(String::from("div")),
                        id: None,
                        class: vec![],
                    })
                ],
                declarations: vec![
                    Declaration {
                        name: "background-color".to_string(),
                        value: Value::Keyword("white".to_string()),
                    },
                    Declaration {
                        name: "border-color".to_string(),
                        value: Value::Keyword("red".to_string()),
                    },
                    Declaration {
                        name: "border-width".to_string(),
                        value: Value::Length(1.0,Unit::Px),
                    },
                    Declaration {
                        name: "color".to_string(),
                        value: Value::Keyword("black".to_string()),
                    },
                ],
            },
            Rule {
                selectors: vec![
                    Selector::Simple(SimpleSelector{
                        tag_name: None,
                        id: None,
                        class: vec![String::from("cool")]
                    })
                ],
                declarations: vec![
                    Declaration {
                        name: "color".to_string(),
                        value: Value::Keyword("green".to_string()),
                    },
                ],
            }
        ]
    };
}


fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn number<'a>() -> Parser<'a, u8, f64> {
    let integer = one_of(b"123456789") - one_of(b"0123456789").repeat(0..) | sym(b'0');
    let frac = sym(b'.') + one_of(b"0123456789").repeat(1..);
    let exp = one_of(b"eE") + one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
    let number = sym(b'-').opt() + integer + frac.opt() + exp.opt();
    number.collect().convert(str::from_utf8).convert(|s|f64::from_str(&s))
}

fn string<'a>() -> Parser<'a, u8, String> {
    let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
        | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
        | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
    let escape_sequence = sym(b'\\') * special_char;
    let string = sym(b'"') * (none_of(b"\\\"") | escape_sequence).repeat(0..) - sym(b'"');
    string.convert(String::from_utf8)
}

fn v2s(v:&Vec<u8>) -> String {
    str::from_utf8(v).unwrap().to_string()
}

fn selector<'a>() -> Parser<'a, u8, Selector>{
    let r
        = space()
        * sym(b'.').opt()
        + is_a(alpha).repeat(1..)
    ;
    r.map(|(class_prefix,name)| {
        if class_prefix.is_none() {
            Selector::Simple(SimpleSelector {
                tag_name: Some(v2s(&name)),
                id: None,
                class: vec![]
            })
        } else {
            Selector::Simple(SimpleSelector {
                tag_name: None,
                id: None,
                class: vec![v2s(&name)]
            })
        }
    })
}

#[test]
fn test_tag_selector() {
    let input = br#"div"#;
    println!("{:?}", selector().parse(input));
}

#[test]
fn test_class_selector() {
    let input = br#".cool"#;
    println!("{:?}", selector().parse(input));
}


fn identifier<'a>() -> Parser<'a, u8, String> {
    let r
        = space()
        + is_a(alpha)
        + (is_a(alphanum) | sym(b'-')).repeat(0..)
        ;
    r.map(|((_,uu),v)| {
        let mut vv = vec![uu];
        vv.extend(&v);
        return String::from_utf8(vv).unwrap();
    })
}
#[test]
fn test_identifier() {
    let input = br"bar";
    println!("{:?}",identifier().parse(input));
}

fn prop_def<'a>() -> Parser<'a, u8, Declaration> {
    let r = space()
        + identifier()
        - (space() - sym(b':') - space())
        + identifier()
        - (space() - sym(b';') - space())
    ;
    r.map(|(((),a),b)| Declaration {
        name: a,
        value: Value::Keyword(b)
    })
}

#[test]
fn test_prop_def() {
    let input = br#"border:black;"#;
    println!("{:?}",prop_def().parse(input))
}
#[test]
fn test_prop_def2() {
    let input = b"border-color:black;";
    println!("{:?}",prop_def().parse(input))
}
#[test]
fn test_prop_def3() {
    let input = b"border-width:1px;";
    println!("{:?}",prop_def().parse(input))
}

/*
#[derive(Debug, PartialEq)]
struct CSSRule {
    selector:String,
    defs:Vec<CSSPropDef>,
}
fn ws_sym<'a>(ch:u8) -> Parser<'a, u8,u8> {
    space() * sym(ch) - space()
}
fn rule<'a>() -> Parser<'a, u8, CSSRule> {
    let r
        = selector()
        - ws_sym(b'{')
        + prop_def().repeat(0..)
        - ws_sym(b'}')
        ;
    r.map(|(sel,value)| CSSRule {
        selector:sel,
        defs:value,
    })
}

#[test]
fn test_css() {
    let input = br#"
      div {
        bar: baz;
        billz: babs;
      }
    "#;
    println!("{:?}", rule().parse(input));
}


fn simplep<'a>() -> Parser<'a, u8, String> {
    is_a(alpha).repeat(1..).map(|c| {
        return String::from("foo")
    })
}

#[test]
fn simple() -> () {
    let mut file = File::open("tests/foo.css").unwrap();
    let mut contents:Vec<u8> = Vec::new();
    file.read_to_end(&mut contents);
    let c2 = contents.as_slice();
    println!("{:?}", simplep().parse(c2));
}
*/
