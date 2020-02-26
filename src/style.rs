/*

https://www.w3.org/TR/CSS2/syndata.html#values



*/

enum Num {
    Integer(i32),
    Number(f32),
}
enum LengthUnit {
    em(Num),
    ex(Num),
    x_height(Num),
    inches(Num),
    cm(Num),
    mm(Num),
    pt(Num),
    pc(Num),
    px(Num),
    per(Num),
}
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

enum Selector {
    Universal(),
    Type(String),
}

struct Rule {
    selector:Selector,
    declarations:Vec<Declaration>,
}

pub fn makeExamples() {

    //make every element use black for the foreground color
    let fg_black = Rule {
        selector: Selector::Universal(),
        declarations: vec![Declaration{
            name:"color".to_string(),
            value:Value::Color(Color::keyword(String::from("black"))),
        }]
    };

    //make every div have a red border and a blue background
    let red_border = Rule {
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

/*    match red_border.value {
        Value::Number(Num) => println!("number"),
        Value::Length(_) => println!("length"),
        Value::Color(_) => println!("color"),
    }*/
//    println!("decl is {} = {}", red_border.name, red_border.value);
}