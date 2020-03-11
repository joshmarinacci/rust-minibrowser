extern crate pom;
use pom::parser::{Parser,is_a,one_of,sym, none_of,seq};
use pom::char_class::alpha;
use std::str::{self, FromStr};
use self::pom::char_class::alphanum;
use std::fs::File;
use std::io::Read;
use crate::net::BrowserError;
use crate::css::Value::{Length, Keyword, HexColor, ArrayValue, StringLiteral};
use crate::css::Unit::Px;
use self::pom::set::Set;
use self::pom::parser::{list, call};


#[derive(Debug, PartialEq)]
pub struct Stylesheet {
    pub(crate) rules: Vec<Rule>,
    pub parent: Option<Box<Stylesheet>>,
}
#[derive(Debug, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}
#[derive(Debug, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector)
}
#[derive(Debug, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}
#[derive(Debug, PartialEq)]
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
    pub(crate) r:u8,
    pub(crate) g:u8,
    pub(crate) b:u8,
    pub(crate) a:u8,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunCallValue {
    pub (crate) name:String,
    pub (crate) arguments: Vec<Value>,
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let Selector::Simple(ref simple) = *self;
        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();
        (a,b,c)
    }
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

pub fn star(term:u8) -> bool {
    term == b'*'
}

fn alpha_string<'a>() -> Parser<'a, u8, String> {
    let r = is_a(alpha).repeat(1..);
    r.map(|str| String::from_utf8(str).unwrap())
}
fn alphanum_string<'a>() -> Parser<'a, u8, String> {
    let r = is_a(alphanum).repeat(1..);
    r.map(|str| String::from_utf8(str).unwrap())
}
fn star_string<'a>() -> Parser<'a, u8, String> {
    let r = sym(b'*');
    r.map(|str|{
        let mut s = String::new();
        s.push(char::from(str));
        s
    })
}
fn class_string<'a>() -> Parser<'a,u8,String> {
    let r = sym(b'.') + alphanum_string();
    r.map(|(dot,str)| {
        let mut s = String::from(str);
        s.insert(0,char::from(dot));
        s
    })
}


fn string_literal<'a>() -> Parser<'a, u8, Value> {
    string().map(|s|StringLiteral(s))
}

#[test]
fn test_string_literal() {
    assert_eq!(string_literal().parse(br#""foo""#),
               Ok(Value::StringLiteral(String::from("foo"))));
}


fn selector<'a>() -> Parser<'a, u8, Selector>{
    let r
        = space()
        + (class_string() | star_string() | alphanum_string())
        - space()
    ;
    r.map(|(_,name)| {
        if name.starts_with(".") {
            Selector::Simple(SimpleSelector {
                tag_name: None,
                id: None,
                class: vec![name]
            })
        } else {
            Selector::Simple(SimpleSelector {
                tag_name: Some(name),
                id: None,
                class: vec![]
            })
        }
    })
}

#[test]
fn test_div_selector() {
    let input = br#"div"#;
    let result = selector().parse(input);
    println!("{:?}", result);
    assert_eq!(Selector::Simple(SimpleSelector{
        tag_name:Some("div".to_string()),
        id: None,
        class: vec![],
    }), result.unwrap())
}

#[test]
fn test_h3_selector() {
    let input = br#"h3"#;
    let result = selector().parse(input);
    println!("{:?}", result);
    assert_eq!(Selector::Simple(SimpleSelector{
        tag_name:Some("h3".to_string()),
        id: None,
        class: vec![],
    }), result.unwrap())
}

#[test]
fn test_class_selector() {
    let input = br#".cool"#;
    let result = selector().parse(input);
    println!("{:?}", result);
    assert_eq!(Selector::Simple(SimpleSelector{
        tag_name:None,
        id: None,
        class: vec![".cool".to_string()],
    }), result.unwrap())
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
        return v2s(&vv)
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
fn test_length_units() {
    assert_eq!(length_unit().parse(br"3px"), Ok(Length(3.0,Unit::Px)));
    assert_eq!(length_unit().parse(br"3em"), Ok(Length(3.0,Unit::Em)));
}

fn funarg<'a>() -> Parser<'a, u8, Value> {
    string_literal() | hexcolor() | length_unit() | keyword()
}

fn funcall<'a>() -> Parser<'a, u8, Value> {
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
}

fn hexcolor<'a>() -> Parser<'a, u8, Value> {
    let p = sym(b'#')
    * one_of(b"0123456789ABCDEFabcdef").repeat(6..7);
    p.map(|mut c| {
        // i32::from_str_radix(v2s(&c),16)
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
fn one_value<'a>() -> Parser<'a, u8, Value> {
    funcall() | hexcolor() | length_unit() | keyword() | string_literal()
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

fn array_value_4<'a>() -> Parser<'a, u8, Value> {
    let t = one_value() - space() + one_value() - space() + one_value() - space() + one_value();
    t.map(|(((v1,v2),v3),v4)|{
        Value::ArrayValue(vec![v1,v2,v3,v4])
    })
}

fn value<'a>() -> Parser<'a, u8, Value> {
    call(array_value_4) | call(array_value_2) |
        call(list_array_value) |
        one_value()
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

fn rule<'a>() -> Parser<'a, u8, Rule> {
    let r
        = list(selector(),sym(b','))
        - ws_sym(b'{')
        + declaration().repeat(0..)
        - ws_sym(b'}')
        ;
    r.map(|(sel, declarations)| Rule {
        selectors: sel,
        declarations,
    })
}

#[test]
fn test_rule() {
    let input = b"div { border-width:1px; }";
    println!("{:#?}",rule().parse(input))
}
fn stylesheet<'a>() -> Parser<'a, u8, Stylesheet> {
    rule().repeat(0..).map(|rules| Stylesheet { rules, parent: None, })
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
                        class: vec![String::from(".cool")]
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

#[test]
fn test_tufte_rules() {
    let mut file = File::open("C:/Users/josh/WebstormProjects/rust-minibrowser2/tests/tufte/tufte.css").unwrap();
    let mut content:Vec<u8>= Vec::new();
    file.read_to_end(&mut content);
    let parsed = stylesheet().parse(content.as_slice()).unwrap();
    println!("parsed {:#?}",parsed);

}


#[derive(Debug, PartialEq)]
struct AtRule {
    name:String,
    value:Option<Value>,
    rules: Vec<Rule>,
}

//https://developer.mozilla.org/en-US/docs/Web/CSS/At-rule
fn at_rule<'a>() -> Parser<'a, u8, AtRule> {
    let p
        = space()
        - sym(b'@')
        + identifier()
        - space()
        // + keyword().opt()
        + string_literal().opt()
        /*
        + funcall().opt()
        + (
             sym(b'{')
             - space()
             + rule().repeat(0..)
             - space()
             - sym(b'}')
             ).opt()
        - sym(b';')
        */
        ;
    p.map(|((_, name), value)|{
        AtRule {
            name,
            value,
            rules: vec![]
        }
    })
}

#[test]
fn test_atrule() {
    assert_eq!(
        at_rule().parse(br#"@charset "UTF-8";"#),
        Ok(AtRule{
            name: String::from("charset"),
            value: Some(StringLiteral(String::from("UTF-8"))),
            rules: vec![]
        }),
    );
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
    /*
    let mut input = br#"@font-face {
    font-family: "et-book";
    src: url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot");
    src: url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot?#iefix") format("embedded-opentype"),
         url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.woff") format("woff"),
         url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.ttf") format("truetype"),
         url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.svg#etbookromanosf") format("svg");
    font-weight: normal;
    font-style: normal;
    font-display: swap;
}
"#;
    // let result = fontface().parse(input.as_ref());
    // println!("{:?}", result);
    // assert_eq!(AtRule {
    //     name:"font-face".to_string(),
    //     declarations: vec![
    //         //src: url("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot")
    //         Declaration { name: String::from("font-family"), value: StringValue(String::from("et-book")) },
    //         Declaration { name: String::from("src"), value: Value::Url(String::from("et-book/et-book-roman-line-figures/et-book-roman-line-figures.eot")) },
    //         Declaration { name: String::from("font-weight"), value: Keyword(String::from("normal")) },
    //         Declaration { name: String::from("font-style"), value: Keyword(String::from("normal")) },
    //         Declaration { name: String::from("font-display"), value: Keyword(String::from("swap")) },
    //     ]
    // },result.unwrap());
    */
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
    let answer = Rule {
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
    };
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
