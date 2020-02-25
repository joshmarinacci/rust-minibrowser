pub enum Elem {
    Block(BlockElem),
    Text(TextElem)
}
pub struct BlockElem {
    pub children: Vec<Elem>,
}

pub struct TextElem {
    pub text:String,
}
