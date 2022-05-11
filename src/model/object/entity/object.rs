pub struct Field{
    pub alias:String,
    pub kind:String,
    pub default:Option<String>,
    pub value:Option<String>,
    pub require:bool,
    pub index:bool,
    pub preview:bool
}

pub struct ObjectType{
    pub fields:Vec<Field>,
    pub kind:String,
    pub alias:String
}

pub struct Object{
    pub(crate) filled:ObjectType,
    pub(crate) hash:String
}