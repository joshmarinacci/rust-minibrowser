/*

https://www.w3.org/TR/CSS2/syndata.html#values



*/


#[allow(dead_code)]
enum Num {
    Integer(i32),
    Number(f32),
}
#[allow(dead_code)]
enum LengthUnit {
    Em(Num),
    Ex(Num),
    X_height(Num),
    Inches(Num),
    CM(Num),
    MM(Num),
    Pt(Num),
//    Pc(Num),
    PX(Num),
    Per(Num),
}

#[allow(dead_code)]
enum Color {
    hex(Num),
    rgb(Num),
    rgba(Num),
    keyword(String),
}

enum Value {
    Number(Num),
    Length(LengthUnit),
    Color(Color),
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
}
/*
//impl for StyleManager {
    fn new() -> StyleManager {

    }
    //look up fg, bg, and border colors
    fn lookupColor(name:String) -> Option<Color> {

    }
    //lookup block width
    fn lookupBlockWidth() -> Option<LengthUnit> {

    }
    //look up font size or other plain numbers
    fn lookupNumber() -> Option<LengthUnit> {

    }
//}
*/
pub fn make_examples() -> StyleManager {

    let mut styles = StyleManager::new();


    //make every element use color:black, width:100%, font-size: 36pt
    let general_styles = Rule {
        selector: Selector::Universal(),
        declarations: vec![
            Declaration{
                name:String::from("color"),
                value:Value::Color(Color::keyword(String::from("black"))),
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
        declarations: vec![Declaration{
            name:"border-color".to_string(),
            value:Value::Color(Color::keyword("red".to_string()))
        },
        Declaration {
            name:"background-color".to_string(),
            value:Value::Color(Color::keyword("blue".to_string()))
        }
        ]
    };
    styles.add_rule(div_styles);
    return styles;
}