extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of,seq};
use pom::char_class::alpha;
use std::collections::HashMap;
use std::str::{self, FromStr};
use self::pom::char_class::alphanum;
use std::fs::File;
use std::io::Read;



#[derive(Debug, PartialEq)]
struct Stylesheet {
    rules: Vec<Rule>,
}
#[derive(Debug, PartialEq)]
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
        return v2s(&vv)
    })
}
#[test]
fn test_identifier() {
    let input = br"bar";
    println!("{:?}",identifier().parse(input));
}

//if px, then turn Unit::px
fn unit<'a>() -> Parser<'a, u8, Unit> {
    seq(&br"px"[0..]).map(|_| Unit::Px)
}

#[test]
fn test_unit() {
    let input = br"px";
    println!("{:?}",unit().parse(input))
}

fn length_unit<'a>() -> Parser<'a, u8, Value> {
    let p = number() + unit();
    p.map(|(v,unit)| {
        Value::Length(v as f32,unit)
    })
}

#[test]
fn test_length_unit() {
    let input = br"3px";
    println!("{:?}",length_unit().parse(input))
}

fn keyword<'a>() -> Parser<'a, u8, Value> {
    let r
        = space()
        + (is_a(alpha)).repeat(0..)
        ;
    r.map(|(_,c)| {
        Value::Keyword(String::from_utf8(c).unwrap())
    })
}

#[test]
fn test_keyword() {
    let input = br"black";
    println!("{:#?}",keyword().parse(input))
}

fn value<'a>() -> Parser<'a, u8, Value> {
    length_unit() | keyword()
}

fn declaration<'a>() -> Parser<'a, u8, Declaration> {
    let r = space()
        + identifier()
        - (space() - sym(b':') - space())
        + value()
        - (space() - sym(b';') - space())
    ;
    r.map(|(((), name), value)| Declaration { name, value })
}

#[test]
fn test_prop_def() {
    let input = br#"border:black;"#;
    println!("{:?}", declaration().parse(input))
}
#[test]
fn test_prop_def2() {
    let input = b"border-color:black;";
    println!("{:?}", declaration().parse(input))
}
#[test]
fn test_prop_def3() {
    let input = b"border-width:1px;";
    println!("{:?}", declaration().parse(input))
}

fn ws_sym<'a>(ch:u8) -> Parser<'a, u8,u8> {
    space() * sym(ch) - space()
}

fn rule<'a>() -> Parser<'a, u8, Rule> {
    let r
        = selector()
        - ws_sym(b'{')
        + declaration().repeat(0..)
        - ws_sym(b'}')
        ;
    r.map(|(sel, declarations)| Rule {
        selectors: vec![sel],
        declarations,
    })
}

#[test]
fn test_rule() {
    let input = b"div { border-width:1px; }";
    println!("{:#?}",rule().parse(input))
}
fn stylesheet<'a>() -> Parser<'a, u8, Stylesheet> {
    rule().repeat(0..).map(|rules| Stylesheet { rules })
}

#[test]
fn test_stylesheet() {
    let input = b"div { border-width:1px; } .cool { color: red; }";
    println!("{:#?}",stylesheet().parse(input))
}

#[test]
fn test_file_load() {
    let mut file = File::open("tests/foo.css").unwrap();
    let mut content:Vec<u8>= Vec::new();
    file.read_to_end(&mut content);
    let parsed = stylesheet().parse(content.as_slice()).unwrap();
    println!("{:#?}", parsed);
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
    assert_eq!(ss,parsed)
}
