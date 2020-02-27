use crate::render::{RenderColor, RED, BLACK, BLUE, WHITE};

/*

https://www.w3.org/TR/CSS2/syndata.html#values

*/

pub enum ColorProps {
    color,
    border_color,
    background_color,
}
impl ColorProps {
    fn to_string(&self) -> &str {
        match self {
            ColorProps::color => "color",
            ColorProps::border_color => "border-color",
            ColorProps::background_color => "background-color",
        }
    }
}


#[allow(dead_code)]
enum Num {
    Integer(i32),
    Number(f32),
}
impl Num {
    fn to_string(&self) -> String {
        match self {
            Num::Integer(v) => format!("{}",v),
            Num::Number(v)  => format!("{}",v),
        }
    }
}
#[allow(dead_code)]
enum LengthUnit {
    Em(Num),
    Ex(Num),
    Xheight(Num),
    Inches(Num),
    CM(Num),
    MM(Num),
    Pt(Num),
//    Pc(Num),
    Px(Num),
    Per(Num),
}
impl LengthUnit {
    fn to_string(&self) -> String {
        match self {
            LengthUnit::Em(v) => format!("{}em",v.to_string()),
            LengthUnit::Per(v) => format!("{}%",v.to_string()),
            LengthUnit::Pt(v) => format!("{}pt",v.to_string()),
            _ => String::from("other unit")
        }
    }
}

#[allow(dead_code)]
enum Color {
    Hex(Num),
    Rgb(Num),
    Rgba(Num),
    Keyword(String),
}
impl Color {
    #[allow(non_snake_case)]
    fn to_RenderColor(&self) -> RenderColor {
        match self {
            Color::Keyword(str) => {
                return match str.as_str() {
                    "blue" => BLUE,
                    "red" => RED,
                    "black" => BLACK,
                    "white" => WHITE,
                    _ => {
                        println!("unknown color keyword {}",str);
                        BLUE
                    }
                }
            }
            _ => {
                println!("other color types not supported yet");
                return BLUE
            }
        }
    }
}

enum Value {
    Number(Num),
    Length(LengthUnit),
    Color(Color),
}
impl Value {
    fn to_string(&self) -> String {
        match &*self {
            Value::Number(num) => num.to_string(),
            Value::Length(len) => len.to_string(),
            Value::Color(col) => {
                match col {
                    Color::Keyword(k) => k.to_string(),
                    _ => "some color".to_string(),
                }
            }
        }
    }
}

struct Declaration {
    name:String,
    value:Value,
}

#[allow(dead_code)]
enum Selector {
    Universal(),
    Type(String),
}
impl Selector {
    fn is_universal(&self) -> bool {
        match *self {
            Selector::Universal() => true,
            _ => false,
        }
    }
    fn to_string(&self) -> &str {
        match &*self {
            Selector::Universal() => "*",
            Selector::Type(txt) => txt.as_str(),
        }
    }
}

#[allow(dead_code)]
struct Rule {
    selector:Selector,
    declarations:Vec<Declaration>,
}

#[allow(dead_code)]
pub struct StyleManager {
    rules: Vec<Rule>,
}


impl StyleManager {
    fn new() -> StyleManager{
        StyleManager {
            rules: Vec::new()
        }
    }
    fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    fn find_prop(&self, prop_name:&str) -> Result<&Declaration, &'static str> {
        for rule in self.rules.iter() {
            for decl in rule.declarations.iter() {
                if decl.name == prop_name {
                    return Ok(decl);
                }
            }
        }
        Err("no prop name found")
    }

    pub fn find_color_prop_enum(&self, name:ColorProps) -> RenderColor {
        let res = self.find_prop(name.to_string());
        match res {
            Ok(decl) => {
                match &decl.value {
                    Value::Color(color) => color.to_RenderColor(),
                    _ => {
                        println!("invalid color type");
                        return BLUE;
                    }
                }
            }
            _ => BLUE
        }
    }

    pub fn dump(&self) {
        for rule in self.rules.iter() {
            println!("rule {}", rule.selector.to_string());
            for decl in rule.declarations.iter() {
                println!("  {} : {};",decl.name,decl.value.to_string());
            }
        }
    }
}

pub fn make_examples() -> StyleManager {

    let mut styles = StyleManager::new();


    //make every element use color:black, width:100%, font-size: 36pt
    let general_styles = Rule {
        selector: Selector::Universal(),
        declarations: vec![
            Declaration{
                name:String::from("color"),
                value:Value::Color(Color::Keyword(String::from("black"))),
            },
            Declaration{
                name:String::from("width"),
                value:Value::Length(LengthUnit::Per(Num::Number(100.0))),
            },
            Declaration {
                name:String::from("font-size"),
                value:Value::Length(LengthUnit::Pt(Num::Number(36.0))),
            }
        ]
    };
    styles.add_rule( general_styles);

    //make every div have a border-color:red and background-color:blue
    let div_styles = Rule {
        selector: Selector::Type(String::from("div")),
        declarations: vec![
            Declaration{
                name:"border-color".to_string(),
                value:Value::Color(Color::Keyword("red".to_string()))
            },
            Declaration {
                name:"background-color".to_string(),
                value:Value::Color(Color::Keyword("white".to_string()))
            }
        ]
    };
    styles.add_rule(div_styles);

    println!("made a bunch of rules");
    styles.dump();
    return styles;
}