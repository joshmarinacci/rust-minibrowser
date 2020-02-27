use serde_json;
use serde_json::{Value};
use std::fs;

pub enum Elem {
    Block(BlockElem),
    Text(TextElem)
}
pub struct BlockElem {
    pub etype:String,
    pub children: Vec<Elem>,
}

pub struct TextElem {
    pub text:String,
}


fn parse_block(json:&Value) -> Elem {
    let rtype = json["type"].as_str().unwrap();
    if rtype == "body" || rtype == "div" {
        println!("parsed {}",rtype);
        let mut block = BlockElem {
            children: Vec::new(),
            etype:rtype.to_string(),
        };
        for child in json["children"].as_array().unwrap() {
            block.children.push(parse_block(&child));
        }
        return Elem::Block(block);
    }

    if rtype == "text" {
        return Elem::Text(TextElem {
            text:json["text"].as_str().unwrap().to_string()
        });
    }

    panic!("found an element type we cant handle")
}

pub fn load_doc(filename:&str) -> Elem {
    let data = fs::read_to_string(filename).expect("file shoudl open");
    let parsed:Value = serde_json::from_str(&data).unwrap();
    println!("parsed the type {}",parsed["type"]);

    return parse_block(&parsed);
}

