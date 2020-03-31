extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of,seq};
use pom::char_class::alpha;
use std::str::{self, FromStr};
use self::pom::char_class::{alphanum};
use std::fs::File;
use std::io::Read;
use crate::net::BrowserError;
use crate::css::Value::{Length, Keyword,  StringLiteral, UnicodeRange, UnicodeCodepoint};
use self::pom::parser::{list, call, take};
use url::Url;
use crate::css::RuleType::Comment;


#[derive(Debug, PartialEq)]
pub struct Stylesheet {
    pub(crate) rules: Vec<RuleType>,
    pub parent: Option<Box<Stylesheet>>,
    pub base_url: Url,
}
#[derive(Debug, PartialEq)]
pub enum RuleType {
    Rule(Rule),
    AtRule(AtRule),
    Comment(String),
}
#[derive(Debug, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, PartialEq)]
pub struct AtRule {
    pub name:String,
    pub value:Option<Value>,
    pub rules: Vec<RuleType>,
}


#[derive(Debug, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
    Ancestor(AncestorSelector),
}
#[derive(Debug, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct AncestorSelector {
    pub ancestor: Box<Selector>,
    pub child: Box<Selector>,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Declaration {
    pub(crate) name: String,
    pub(crate) value: Value,
}
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
    HexColor(String),
    ArrayValue(Vec<Value>),
    FunCall(FunCallValue),
    StringLiteral(String),
    UnicodeCodepoint(i32),
    UnicodeRange(i32,i32),
    Number(f32),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    Px,
    Em,
    Per,
    Rem,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Color {
    pub r:u8,
    pub g:u8,
    pub b:u8,
    pub a:u8,
}
impl Color {
    pub fn from_hex(str:&str) -> Self {
        let n = i32::from_str_radix(&str[1..], 16).unwrap();
        let r = (n >> 16) & 0xFF;
        let g = (n >> 8) & 0xFF;
        let b = (n/*>>0*/) & 0xFF;
        Self {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: 255
        }
    }
    pub fn to_array(&self) -> [f32;4]{
        [(self.r as f32)/255.0, (self.g as f32)/255.0, (self.b as f32)/255.0, (self.a as f32)/255.0]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunCallValue {
    pub (crate) name:String,
    pub (crate) arguments: Vec<Value>,
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        if let Selector::Simple(ref simple) = *self {
            let a = simple.id.iter().count();
            let b = simple.class.len();
            let c = simple.tag_name.iter().count();
            return (a, b, c)
        }
        if let Selector::Ancestor(ref anc) = *self {
            return anc.ancestor.specificity();
        }
        panic!("unknown selector type");
    }
}


fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}
fn space1<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(1..).discard()
}

fn number<'a>() -> Parser<'a, u8, f64> {
    let integer = one_of(b"123456789") - one_of(b"0123456789").repeat(0..) | sym(b'0');
    let frac = sym(b'.') + one_of(b"0123456789").repeat(1..);
    // let exp = one_of(b"eE") + one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
    let number = sym(b'-').opt() + integer.opt() + frac.opt();// + exp.opt();
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
fn single_quote_string<'a>() -> Parser<'a, u8, String> {
    (sym(b'\'') * none_of(b"'").repeat(0..) - sym(b'\'')).map(|v|v2s(&v))
}

fn v2s(v:&[u8]) -> String {
    str::from_utf8(v).unwrap().to_string()
}

pub fn star(term:u8) -> bool {
    term == b'*'
}

fn alphanum_string<'a>() -> Parser<'a, u8, Selector> {
    let r = is_a(alphanum).repeat(1..);
    r.map(|str| {
        Selector::Simple(SimpleSelector{
            tag_name: Some(v2s(&str)),
            id: None,
            class: vec![]
        })
    })
}
fn star_string<'a>() -> Parser<'a, u8, Selector> {
    let r = sym(b'*');
    r.map(|str|{
        Selector::Simple(SimpleSelector{
            tag_name: Some(char::from(str).to_string()),
            id: None,
            class: vec![]
        })
    })
}
fn class_string<'a>() -> Parser<'a,u8,Selector> {
    let r = sym(b'.') + is_a(alphanum).repeat(1..);
    r.map(|(_dot,str)| {
        Selector::Simple(SimpleSelector{
            tag_name: None,
            id: None,
            class: vec![v2s(&str)]
        })
    })
}
fn ancestor<'a>() -> Parser<'a,u8,Selector> {
    let r = alphanum_string() - space1() + alphanum_string();
    r.map(|(a,b)| Selector::Ancestor(AncestorSelector{
        ancestor: Box::new(a),
        child: Box::new(b),
    }))
}


fn string_literal<'a>() -> Parser<'a, u8, Value> {
    (single_quote_string() | string()).map(StringLiteral)
}

#[test]
fn test_string_literal() {
    assert_eq!(string_literal().parse(br#""foo""#),
               Ok(Value::StringLiteral(String::from("foo"))));
}


fn selector<'a>() -> Parser<'a, u8, Selector>{
    let r
        = space()
        + (ancestor() | class_string() | star_string() | alphanum_string())
        - space()
    ;
    r.map(|(_, selector)| selector)
}

#[test]
fn test_selectors() {
    assert_eq!(selector().parse(b"div"),
               Ok(Selector::Simple(SimpleSelector {
                   tag_name: Some("div".to_string()),
                   id: None,
                   class: vec![],
               })));
    assert_eq!(selector().parse(b"h3"),
               Ok(Selector::Simple(SimpleSelector {
                   tag_name: Some("h3".to_string()),
                   id: None,
                   class: vec![],
               })));
    assert_eq!(selector().parse(b".cool"),
               Ok(Selector::Simple(SimpleSelector {
                   tag_name: None,
                   id: None,
                   class: vec![String::from("cool")],
               })));
}
#[test]
fn test_ancestor_selector() {
    assert_eq!(selector().parse(b"a b"),
               Ok(Selector::Ancestor(AncestorSelector{
                   ancestor:Box::new(Selector::Simple(SimpleSelector{
                       tag_name:Some(String::from("a")),
                       id: None,
                       class: vec![],
                   })),
                   child:Box::new(Selector::Simple(SimpleSelector{
                       tag_name:Some(String::from("b")),
                       id: None,
                       class: vec![],
                   })),
               })));
}

#[test]
fn test_all_selector() {
    let input = br#"*"#;
    let result = selector().parse(input);
    println!("{:?}", result);
    assert_eq!(Selector::Simple(SimpleSelector{
        tag_name:Some("*".to_string()),
        id: None,
        class: vec![],
    }), result.unwrap())
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
        v2s(&vv)
    })
}
#[test]
fn test_identifier() {
    let input = br"bar";
    println!("{:?}",identifier().parse(input));
}

fn unit_px<'a>() -> Parser<'a, u8, Unit> {
    seq(b"px").map(|_| Unit::Px)
}
fn unit_per<'a>() -> Parser<'a, u8, Unit> {
    seq(b"%").map(|_| Unit::Per)
}
fn unit_rem<'a>() -> Parser<'a, u8, Unit> {
    seq(b"rem").map(|_| Unit::Rem)
}
fn unit_em<'a>() -> Parser<'a, u8, Unit> {
    seq(b"em").map(|_| Unit::Em)
}
fn unit<'a>() -> Parser<'a, u8, Unit> {
    unit_per() | unit_px() | unit_rem() | unit_em()
}
#[test]
fn test_unit() {
    assert_eq!(unit().parse(b"px"),Ok(Unit::Px));
    assert_eq!(unit().parse(b"em"),Ok(Unit::Em));
}

fn length_unit<'a>() -> Parser<'a, u8, Value> {
    let p = number() + unit();
    p.map(|(v,unit)| {
        Value::Length(v as f32,unit)
    })
}

#[test]
fn test_length_units() {
    assert_eq!(length_unit().parse(br"3px"), Ok(Length(3.0,Unit::Px)));
    assert_eq!(length_unit().parse(br"3em"), Ok(Length(3.0,Unit::Em)));
    assert_eq!(length_unit().parse(br"0.3em"), Ok(Length(0.3,Unit::Em)));
    assert_eq!(length_unit().parse(br".3em"), Ok(Length(0.3,Unit::Em)));
}

fn funarg<'a>() -> Parser<'a, u8, Value> {
    string_literal() | hexcolor() | length_unit() | keyword()
}

fn normal_funcall<'a>() -> Parser<'a, u8, Value> {
    let p
        = space()
        + identifier()
        - space()
        - sym(b'(')
        - space()
        + list(funarg(),space() - sym(b',') - space())
        -space()
        - sym(b')');
    p.map(|((_,name), arguments)| Value::FunCall(FunCallValue{
        name,
        arguments
    }))
}
//format('woff2')
fn format_follower<'a>() -> Parser<'a, u8, Value> {
    let p = seq(b"format")
        - sym(b'(')
        + (string_literal() | url())
        - sym(b')');
    p.map(|(_a,b)|Value::FunCall(FunCallValue{
        name: "format".to_string(),
        arguments: vec![b]
    }))
}
fn url_funcall<'a>() -> Parser<'a, u8, Value> {
    let p
        = space()
        - seq(b"url")
        - space()
        - sym(b'(')
        + (string_literal() | url())
        - sym(b')')
        - space()
        + format_follower().opt()
        ;
    p.map(|((_a,url),_format)| Value::FunCall(FunCallValue{
        name: "url".to_string(),
        arguments: vec![url]
    }))

}

fn funcall<'a>() -> Parser<'a, u8, Value> {
    url_funcall() | normal_funcall()
}

#[test]
fn test_funcall_value() {
    assert_eq!(funcall().parse(br"foo()"),
               Ok(Value::FunCall(FunCallValue{ name: "foo".parse().unwrap(), arguments: vec![] })));
    assert_eq!(funcall().parse(br"foo(keyword, keyword)"),
               Ok(Value::FunCall(FunCallValue{
                   name: String::from("foo"),
                   arguments: vec![
                       Keyword(String::from("keyword")),
                       Keyword(String::from("keyword")),
                   ] })
               ));
    assert_eq!(funcall().parse(br"foo(#fffff8,#fffff8)"),
               Ok(Value::FunCall(FunCallValue{
                   name: String::from("foo"),
                   arguments: vec![
                       Value::HexColor(String::from("#fffff8")),
                       Value::HexColor(String::from("#fffff8")),
                   ] })
               ));
    assert_eq!(funcall().parse(br" foo ( #fffff8 , #fffff8 ) "),
               Ok(Value::FunCall(FunCallValue{
                   name: String::from("foo"),
                   arguments: vec![
                       Value::HexColor(String::from("#fffff8")),
                       Value::HexColor(String::from("#fffff8")),
                   ] })
               ));
    assert_eq!(funcall().parse(br" linear-gradient ( #fffff8 , #fffff8 ) "),
               Ok(Value::FunCall(FunCallValue{
                   name: String::from("linear-gradient"),
                   arguments: vec![
                       Value::HexColor(String::from("#fffff8")),
                       Value::HexColor(String::from("#fffff8")),
                   ] })
               ));
    assert_eq!(declaration().parse(br"foo:linear-gradient(#fffff8,#fffff8);"),
               Ok(Declaration {
                   name: String::from("foo"),
                   value:Value::FunCall(FunCallValue{
                       name: String::from("linear-gradient"),
                       arguments: vec![
                           Value::HexColor(String::from("#fffff8")),
                           Value::HexColor(String::from("#fffff8")),
                       ],
                   })
               }
               ));
    //check url with double quotes
    assert_eq!(declaration().parse(br#"foo:url("https://www.google.com/");"#),
               Ok(Declaration {
                   name: String::from("foo"),
                   value:Value::FunCall(FunCallValue{
                       name: String::from("url"),
                       arguments: vec![
                           Value::StringLiteral(String::from("https://www.google.com/")),
                       ],
                   })
               }
               ));
    //check url with single quotes
    assert_eq!(declaration().parse(br"foo:url('https://www.google.com/');"),
               Ok(Declaration {
                   name: String::from("foo"),
                   value:Value::FunCall(FunCallValue{
                       name: String::from("url"),
                       arguments: vec![
                           Value::StringLiteral(String::from("https://www.google.com/")),
                       ],
                   })
               }
               ));
    //check url with no quotes
    assert_eq!(declaration().parse(br"foo:url(https://www.google.com/);"),
               Ok(Declaration {
                   name: String::from("foo"),
                   value:Value::FunCall(FunCallValue{
                       name: String::from("url"),
                       arguments: vec![
                           Value::StringLiteral(String::from("https://www.google.com/")),
                       ],
                   })
               }
               ));
}

fn simple_number<'a>() -> Parser<'a, u8, Value> {
    let p = one_of(b"0123456789").repeat(1..);
    p.map(|v|{
        let s = v2s(&v);
        let vv = i32::from_str_radix(&s,10).unwrap() as f32;
        Value::Number(vv)
    })
}
fn hexcolor<'a>() -> Parser<'a, u8, Value> {
    let p = sym(b'#')
        + (  one_of(b"0123456789ABCDEFabcdef").repeat(6..7)
            | one_of(b"0123456789ABCDEFabcdef").repeat(3..4));
    p.map(|(a,mut c)| {
        c.insert(0,b'#');
        Value::HexColor(v2s(&c).to_lowercase())
    })
}

#[test]
fn test_hexcolor() {
    let input = br"#4455fF";
    let result = hexcolor().parse(input);
    println!("{:?}", result);
    assert_eq!( Value::HexColor("#4455FF".to_lowercase()), result.unwrap());
    assert_eq!( Ok(Value::HexColor("#333".to_lowercase())), hexcolor().parse(br"#333"));
}


fn keyword<'a>() -> Parser<'a, u8, Value> {
    let r
        = space()
        + (is_a(|term:u8| {
            (term >= 0x41 && term < 0x5A) || (term >= 0x61 && term <= 0x7A) || (term == '-' as u8)
            })).repeat(1..)
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
#[test]
fn test_keyword_dash() {
    let input = b"inline-block";
    let result = keyword().parse(input);
    println!("{:?}", result);
    assert_eq!( Value::Keyword("inline-block".to_lowercase()), result.unwrap());
}

fn url<'a>() -> Parser<'a, u8, Value> {
    let p = none_of(b")>").repeat(1..);
    p.map(|s|{
        Value::StringLiteral(v2s(&s))
    })
}

fn hex4<'a>() -> Parser<'a,u8,i32> {
    one_of(b"0123456789ABCDEFabcdef").repeat(4).map(|c| i32::from_str_radix(&v2s(&c),16).unwrap())
}

fn unicode_codepoint<'a>() -> Parser<'a, u8, Value> {
    (space() * seq(b"U+") * hex4()).map(Value::UnicodeCodepoint)
}
#[test]
fn test_unicode_codepoint() {
    assert_eq!(one_value().parse(b"U+0100"),Ok(Value::UnicodeCodepoint(0x100)));
    assert_eq!(one_value().parse(b" U+0100"),Ok(Value::UnicodeCodepoint(0x100)));
}

fn unicode_range<'a>() -> Parser<'a, u8, Value> {
    // U+0100-024F
    (space() - seq(b"U+") + hex4() - sym(b'-')+hex4()).map(|((_,a),b)|Value::UnicodeRange(a,b))
}
#[test]
fn test_unicode_range() {
    assert_eq!(one_value().parse(b"U+0100-024F"),Ok(Value::UnicodeRange(0x100, 0x24f)));
}


fn one_value<'a>() -> Parser<'a, u8, Value> {
    unicode_range() | unicode_codepoint() | funcall() | hexcolor() | length_unit() | keyword() | string_literal() | simple_number()
}

fn list_array_value<'a>() -> Parser<'a, u8, Value> {
    let p = list(one_value(), sym(b','));
    // let p = (one_value() + (space() - sym(b',') + one_value()).repeat(0..));
        p.map(|a| {
            if a.len() == 1 {
                a[0].clone()
            } else {
                Value::ArrayValue(a)
            }
        })
}
fn array_value_2<'a>() -> Parser<'a, u8, Value> {
    let t = one_value() - space() + one_value();
    t.map(|(v1,v2)|{
        Value::ArrayValue(vec![v1,v2])
    })
}

fn array_value_3<'a>() -> Parser<'a, u8, Value> {
    let t = one_value() - space() + one_value() - space() + one_value();
    t.map(|((v1,v2),v3)|{
        Value::ArrayValue(vec![v1,v2,v3])
    })
}

fn array_value_4<'a>() -> Parser<'a, u8, Value> {
    let t = one_value() - space() + one_value() - space() + one_value() - space() + one_value();
    t.map(|(((v1,v2),v3),v4)|{
        Value::ArrayValue(vec![v1,v2,v3,v4])
    })
}
#[test]
fn test_list_array_values() {
    assert_eq!(array_value_2().parse(b"3px 4px"),
               Ok(Value::ArrayValue(vec![Value::Length(3.0,Unit::Px), Value::Length(4.0,Unit::Px)])));
    assert_eq!(array_value_2().parse(b"3em 4.0rem"),
               Ok(Value::ArrayValue(vec![Value::Length(3.0,Unit::Em), Value::Length(4.0,Unit::Rem)])));
    assert_eq!(array_value_2().parse(b"0.3em 0.4rem"),
               Ok(Value::ArrayValue(vec![Value::Length(0.3,Unit::Em), Value::Length(0.4,Unit::Rem)])));
    assert_eq!(array_value_2().parse(b".3em 0.4rem"),
               Ok(Value::ArrayValue(vec![Value::Length(0.3,Unit::Em), Value::Length(0.4,Unit::Rem)])));
    assert_eq!(array_value_3().parse(b"1px solid black"),
               Ok(Value::ArrayValue(vec![Value::Length(1.0,Unit::Px),
                                         Value::Keyword(String::from("solid")),
                                         Value::Keyword(String::from("black"))])));
    assert_eq!(array_value_3().parse(b"1px solid #cccccc"),
               Ok(Value::ArrayValue(vec![Value::Length(1.0,Unit::Px),
                                         Value::Keyword(String::from("solid")),
                                         Value::HexColor(String::from("#cccccc"))])));
    assert_eq!(value().parse(b"1px solid #cccccc"),
               Ok(Value::ArrayValue(vec![Value::Length(1.0,Unit::Px),
                                         Value::Keyword(String::from("solid")),
                                         Value::HexColor(String::from("#cccccc"))])));
}


fn value<'a>() -> Parser<'a, u8, Value> {
    call(array_value_4)
        | call(array_value_3)
        | call(array_value_2)
        | call(list_array_value)
        | one_value()
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
#[test]
fn test_prop_def4() {
    let input = b"border-color:#ff00aa;";
    let result = declaration().parse(input);
    println!("{:?}", result);
    assert_eq!(Declaration {
        name: "border-color".to_string(),
        value: Value::HexColor("#ff00aa".to_lowercase())
    },result.unwrap());
    println!("{:?}", declaration().parse(input))
}

fn ws_sym<'a>(ch:u8) -> Parser<'a, u8,u8> {
    space() * sym(ch) - space()
}

fn rule<'a>() -> Parser<'a, u8, RuleType> {
    let r
        = list(selector(),sym(b','))
        - ws_sym(b'{')
        - comment().opt()
        + declaration().repeat(0..)
        - comment().opt()
        - ws_sym(b'}')
        ;
    r.map(|(sel, declarations)| RuleType::Rule(Rule {
        selectors: sel,
        declarations,
    }))
}

fn comment<'a>() -> Parser<'a, u8, RuleType> {
    let p
        =
        space()
        - seq(b"/*")
        + (!seq(b"*/") * take(1)).repeat(0..)
        + seq(b"*/");
    p.map(|((_a,c),_b)| {
        let mut s:Vec<u8> = Vec::new();
        for cc in c {
            s.push(cc[0]);
        }
        RuleType::Comment(v2s(&s))
    })
}
#[test]
fn test_comment() {
    assert_eq!(comment().parse(b"/* a cool comment */"),
               Ok(Comment(String::from(" a cool comment "))));
}

#[test]
fn test_rule() {
    let input = b"div { border-width:1px; }";
    println!("{:#?}",rule().parse(input))
}
fn stylesheet<'a>() -> Parser<'a, u8, Stylesheet> {
    (comment() | rule() | import_rule() | at_rule()).repeat(0..).map(|rules| Stylesheet {
        rules,
        parent: None,
        base_url: Url::parse("https://www.mozilla.com/").unwrap()
    })
}

#[test]
fn test_stylesheet() {
    let input = b"div { border-width:1px; } .cool { color: red; }";
    println!("{:#?}",stylesheet().parse(input))
}

#[test]
fn test_font_style() {
    let input = b"div { font-size: 18px; }";
    println!("{:#?}",stylesheet().parse(input))
}

pub fn parse_stylesheet_from_buffer(content:Vec<u8>) -> Result<Stylesheet, BrowserError> {
    Ok(stylesheet().parse(content.as_slice())?)
}
pub fn parse_stylesheet_from_bytestring(content:&[u8]) -> Result<Stylesheet, BrowserError> {
    Ok(stylesheet().parse(content)?)
}
pub fn parse_stylesheet(text:&str) -> Result<Stylesheet, BrowserError> {
    Ok(stylesheet().parse(text.as_ref())?)
}

#[test]
fn test_file_load() {
    let mut file = File::open("tests/foo.css").unwrap();
    let mut content:Vec<u8>= Vec::new();
    file.read_to_end(&mut content);
    let parsed = stylesheet().parse(content.as_slice()).unwrap();
    println!("{:#?}", parsed);
    let ss = Stylesheet {
        parent: None,
        rules: vec![
            RuleType::Rule(
            Rule {
                selectors: vec![
                    Selector::Simple(SimpleSelector{
                        tag_name: Some(String::from("body")),
                        id: None,
                        class: vec![],
                    }),
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
            }),
            RuleType::Rule(
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
            )
        ],
        base_url: Url::parse("https://www.mozilla.com/").unwrap()
    };
    assert_eq!(ss,parsed)
}

#[test]
fn test_tufte_rules() {
    let mut file = File::open("tests/tufte/tufte.css").unwrap();
    let mut content:Vec<u8>= Vec::new();
    file.read_to_end(&mut content);
    let parsed = stylesheet().parse(content.as_slice()).unwrap();
    println!("parsed {:#?}",parsed);

}

fn import_rule<'a>() -> Parser<'a, u8, RuleType> {
    let p =
            - space()
            - sym(b'@')
            + identifier()
            - space()
            - seq(b"url")
            - sym(b'(')
            + url()
            - sym(b')')
            - sym(b';')
        ;
    p.map(|( (_a,name), url)| {
        RuleType::AtRule(AtRule {
            name,
            value: Some(Value::FunCall(FunCallValue{
                name: String::from("url"),
                arguments: vec![url]
            })),
            rules: vec![]
        })
    })
}


#[test]
fn test_import_rule() {
    let input = br#"http://fonts.googleapis.com/css?family=Lato"#;
    println!("{:#?}", url().parse(input));
    // let input = br#"url(http://fonts.googleapis.com/css?family=Lato)"#;
    // println!("{:#?}", funcall().parse(input));
    let input = br#"@import url(http://fonts.googleapis.com/css?family=Lato);"#;

    assert_eq!(import_rule().parse(input),Ok(RuleType::AtRule(AtRule{
        name: "import".to_string(),
        value: Some(Value::FunCall(FunCallValue{
            name: "url".to_string(),
            arguments: vec![Value::StringLiteral(String::from("http://fonts.googleapis.com/css?family=Lato"))]
        })),
        rules: vec![]
    })));

    let ss = stylesheet().parse(input);
    println!("{:#?}",ss);

    assert_eq!(stylesheet().parse(input),Ok(
        Stylesheet{
            rules: vec![
                RuleType::AtRule(AtRule{
                    name: "import".to_string(),
                    value: Some(Value::FunCall(FunCallValue{
                        name: "url".to_string(),
                        arguments: vec![Value::StringLiteral(String::from("http://fonts.googleapis.com/css?family=Lato"))]
                    })),
                    rules: vec![]
                })
            ],
            parent: None,
            base_url: Url::parse("https://www.mozilla.com/").unwrap()
        }
    ));
}


//https://developer.mozilla.org/en-US/docs/Web/CSS/At-rule
fn at_rule<'a>() -> Parser<'a, u8, RuleType> {
    let p
        = space()
        - sym(b'@')
        + identifier()
        - space()
        + keyword().opt()
        - space()
        + string_literal().opt()
        //+ funcall().opt()
        // + (
        //      sym(b'{')
        - space()
        + rule().opt()
        - space()
             // - sym(b'}')
             // ).opt()
        - sym(b';').opt()

        ;
    p.map(|((((_,name),kw),value), rule)|{
        //we are ignoring the keyword currently
        if let Some(rt) = rule {
            RuleType::AtRule(AtRule { name, value, rules:vec![rt]})
        } else {
            RuleType::AtRule(AtRule {  name,  value,  rules:vec![] })
        }
    })
}

#[test]
fn test_atrules() {
    assert_eq!(
        at_rule().parse(br#"@charset "UTF-8";"#),
        Ok(RuleType::AtRule(AtRule{
            name: String::from("charset"),
            value: Some(StringLiteral(String::from("UTF-8"))),
            rules: vec![]
        })),
    );
    assert_eq!(
        at_rule().parse(br#"@page { size: letter; margin: 1in;  }"#),
        Ok(RuleType::AtRule(AtRule{
            name: String::from("page"),
            value: None,
            rules: vec![RuleType::Rule(Rule {
                selectors: vec![],
                declarations: vec![
                    Declaration { name: String::from("size"),
                        value:Value::Keyword(String::from("letter"))},
                    Declaration {
                        name: String::from("margin"),
                        value: Value::ArrayValue(vec![Value::Number(1.0), Keyword(String::from("in"))]),
                    }
                ]
            })]
        })),
    );
    assert_eq!(
        at_rule().parse(br#"@media screen { body { margin: 3em; }}"#),
        Ok(RuleType::AtRule(AtRule{
            name: String::from("media"),
            value: None,
            rules: vec![]
        }))
    );


    assert_eq!(
        stylesheet().parse(br#"@charset "UTF-8";
/*foo*/
@font-face {
}
"#),
    Ok(Stylesheet {
        rules: vec![
            RuleType::AtRule(AtRule{
                name: "charset".to_string(),
                value: Some(Value::StringLiteral(String::from("UTF-8"))),
                rules: vec![]
            }),
            RuleType::Comment(String::from("foo")),
            RuleType::AtRule(AtRule{
                name: "font-face".to_string(),
                value: None,
                rules: vec![
                    RuleType::Rule(Rule{
                        selectors: vec![],
                        declarations: vec![]
                    })
                ]
            })
        ],
        parent: None,
        base_url: Url::parse("https://www.mozilla.com/").unwrap()
    }));


}

#[test]
fn test_fontface() {
    assert_eq!(Ok(Value::FunCall(FunCallValue{
        name: "url".to_string(),
        arguments: vec![
            StringLiteral(String::from("foo"))
        ]
    })),
           funcall().parse(br#"url("foo")"#));

    assert_eq!(Ok(Declaration{
        name: String::from("src"),
        value: Value::FunCall(FunCallValue{
            name: String::from("url"),
            arguments: vec![
                Value::StringLiteral(String::from("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot"))
            ]
        })
    }),
               declaration().parse(br#"src: url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot");"#));

    let mut input = br#"@font-face {
                font-family: "et-book";
                src: url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot");
                font-weight: normal;
                font-style: normal;
                font-display: swap;
            }
            "#;
    let result = stylesheet().parse(input.as_ref());
    println!("{:?}", result);
    assert_eq!(Ok(Stylesheet {
        rules: vec![
            RuleType::AtRule(AtRule {
            name: "font-face".to_string(),
            value: None,
            rules: vec![RuleType::Rule(Rule {
                selectors: vec![],
                declarations: vec![
                    Declaration { name: String::from("font-family"), value: Value::StringLiteral(String::from("et-book")) },
                    Declaration {
                        name: String::from("src"),
                        value: Value::FunCall(FunCallValue {
                            name: "url".to_string(),
                            arguments: vec![
                                Value::StringLiteral(String::from("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot")),
                            ]
                        })
                    },
                    Declaration { name: String::from("font-weight"), value: Keyword(String::from("normal")) },
                    Declaration { name: String::from("font-style"), value: Keyword(String::from("normal")) },
                    Declaration { name: String::from("font-display"), value: Keyword(String::from("swap")) },
                ]
            })]
        })],
        parent: None,
        base_url: Url::parse("https://www.mozilla.com/").unwrap(),
    }
    ),result);
}

#[test]
fn test_percentage() {
    assert_eq!(Length(100.0,Unit::Per),
               length_unit().parse(br"100%").unwrap());
    assert_eq!(Declaration{
        name: String::from("width"),
        value: (Value::Length(100.0, Unit::Per))
    },
               declaration().parse(br"width:100%;").unwrap());
}

#[test]
fn test_rem() {
    assert_eq!(Length(40.0,Unit::Rem),
               length_unit().parse(br"40rem").unwrap());
    assert_eq!(Length(40.0,Unit::Rem),
               length_unit().parse(br"40.0rem").unwrap());
    assert_eq!(Declaration{
        name: String::from("width"),
        value: (Value::Length(99.90, Unit::Rem))
    },
               declaration().parse(br"width:99.9rem;").unwrap());
}

#[test]
fn test_multiple_selectors() {
    let answer = RuleType::Rule(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector{
                tag_name: Some(String::from("a")),
                id: None,
                class: vec![]
            }),
            Selector::Simple(SimpleSelector{
                tag_name: Some(String::from("b")),
                id: None,
                class: vec![]
            })
        ],
        declarations: vec![
            Declaration{ name: String::from("foo"), value: Keyword(String::from("bar")) }
        ]
    });
    assert_eq!(answer, rule().parse(br"a,b { foo: bar; }").unwrap());
    assert_eq!(answer, rule().parse(br" a , b{ foo: bar; }").unwrap());
}
#[test]
fn test_child_selector() {
    let input = br"a > b { foo: bar; }";
}
#[test]
fn test_simple_pseudo_selector() {
    let input = br"a:hover { foo: bar; }";
}

#[test]
fn test_not_pseudo_selector() {
    let input = br"li:not(:first-child) { foo: bar; }";
}


#[test]
fn test_four_part_margin() {
    println!("parsed {:#?}", value().parse(b"1px 2px 3px 4px"));
    println!("parsed {:#?}", declaration().parse(b"margin: 1px 2px 3px 4px;"));
    let answer = Declaration {
        name: String::from("margin"),
        value: Value::ArrayValue(vec![
            Length(1.0,Unit::Px),
            Length(2.0,Unit::Px),
            Length(3.0,Unit::Px),
            Length(4.0,Unit::Px),
        ])
    };
    assert_eq!(answer, declaration().parse(b"margin: 1px 2px 3px 4px;").unwrap());
    println!("parsed {:#?}", declaration().parse(b"margin: 1px 2px 3px 4em;"));
    let answer = Declaration {
        name: String::from("margin"),
        value: Value::ArrayValue(vec![
            Length(1.0,Unit::Px),
            Length(2.0,Unit::Px),
            Length(3.0,Unit::Px),
            Length(4.0,Unit::Em),
        ])
    };
    assert_eq!(answer, declaration().parse(b"margin: 1px 2px 3px 4em;").unwrap());
}
#[test]
fn test_two_part_margin() {
    println!("parsed {:#?}", array_value_2().parse(b"1px 2px"));
    println!("parsed {:#?}",value().parse(b"1px 2px"));
    println!("parsed {:#?}",value().parse(b"1px 2px"));
    let answer = Declaration {
        name: String::from("margin"),
        value: Value::ArrayValue(vec![
            Length(1.0,Unit::Px),
            Length(2.0,Unit::Px),
        ])
    };
    assert_eq!(answer, declaration().parse(b"margin: 1px 2px;").unwrap());
}
#[test]
fn test_one_part_margin() {
    let answer = Declaration {
        name: String::from("margin"),
        value: Length(1.0,Unit::Px)
    };
    assert_eq!(answer, declaration().parse(b"margin: 1px;").unwrap());
}

#[test]
fn test_funcall_dec() {
    assert_eq!(Declaration{
        name: String::from("background"),
        value: Value::FunCall(FunCallValue{
            name: String::from("linear-gradient"),
            arguments: vec![
                Value::HexColor(String::from("#fffff8")),
                Value::HexColor(String::from("#fffff8")),
            ]
        })
    },
        declaration().parse(b"background: linear-gradient(#fffff8, #fffff8);").unwrap()
    )
}
#[test]
fn test_linear_gradient() {
    assert_eq!(Ok(Declaration{
        name: String::from("background"),
        value: Value::ArrayValue(vec![
            Value::FunCall(FunCallValue{
                name: String::from("linear-gradient"),
                arguments: vec![
                    Value::HexColor(String::from("#fffff8")),
                    Value::HexColor(String::from("#fffff8")),
                ]
            }),
            Value::FunCall(FunCallValue{
                name: String::from("linear-gradient"),
                arguments: vec![
                    Value::HexColor(String::from("#fffff8")),
                    Value::HexColor(String::from("#fffff8")),
                ]
            }),
            Value::FunCall(FunCallValue{
                name: String::from("linear-gradient"),
                arguments: vec![
                    Value::Keyword(String::from("currentColor")),
                    Value::Keyword(String::from("currentColor")),
                ]
            }),
        ])
    }),
       declaration().parse(br"background: linear-gradient(#fffff8, #fffff8), linear-gradient(#fffff8, #fffff8), linear-gradient(currentColor, currentColor);")
    );
}

#[test]
fn test_keyword_list() {
    println!("keyword {:#?}", keyword().parse(b"foo"));
    println!("keyword {:#?}", keyword().parse(b"foo-bar"));
    println!("list value {:#?}", value().parse(b"foo-bar,baz-zoo"));
    println!("decl {:#?}", declaration().parse(b"blah:foo-bar,baz-zoo;"));
    let answer = Declaration {
        name: String::from("background-repeat"),
        value: Value::ArrayValue(vec![
            Keyword(String::from("no-repeat")),
            Keyword(String::from("no-repeat")),
            Keyword(String::from("repeat-x")),
        ])
    };
    assert_eq!(answer, declaration().parse(b"background-repeat:no-repeat,no-repeat,repeat-x;").unwrap());
    assert_eq!(answer, declaration().parse(b"background-repeat: no-repeat, no-repeat, repeat-x;").unwrap());
}

#[test]
fn test_font_weight() {
    assert_eq!(
        declaration().parse(br#"font-weight: normal;"#),
        Ok(Declaration{
            name: String::from("font-weight"),
            value: Value::Keyword(String::from("normal")),
        }),
    );
    assert_eq!(
        declaration().parse(br#"font-weight: 400;"#),
        Ok(Declaration{
            name: String::from("font-weight"),
            value: Value::Number(400.0),
        }),
    );
}

#[test]
fn test_list() {
    let input = b"U+0100-024F, U+0259";
    assert_eq!(list_array_value().parse(input),Ok(Value::ArrayValue(vec![UnicodeRange(0x0100,0x024f),UnicodeCodepoint(0x0259)])))
}

#[test]
fn test_gfonts() {
    let input = br#"
/* latin-ext */
@font-face {
  font-family: 'Lato';
  font-style: normal;
  font-weight: 400;
  src: local('Lato Regular'), local('Lato-Regular'), url(https://fonts.gstatic.com/s/lato/v16/S6uyw4BMUTPHjxAwXiWtFCfQ7A.woff2) format('woff2');
  unicode-range: U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF;
}
/* latin */
@font-face {
  font-family: 'Lato';
  font-style: normal;
  font-weight: 400;
  src: local('Lato Regular'), local('Lato-Regular'), url(https://fonts.gstatic.com/s/lato/v16/S6uyw4BMUTPHjx4wXiWtFCc.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}
"#;

    println!("{:#?}",stylesheet().parse(input));
}

#[test]
fn test_tufte_css() {
    let mut file = File::open("tests/tufte/tufte.css").unwrap();
    let mut content:Vec<u8>= Vec::new();
    file.read_to_end(&mut content);
    let parsed = stylesheet().parse(content.as_slice()).unwrap();
    println!("parsed the stylesheet {:#?}",parsed);
}
