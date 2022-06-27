use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,  Clone, PartialEq)]
pub struct Dictionary{
    pub id:Option<String>,
    pub group:DictionaryGroup,
    pub name:String,
    pub alias:String
}

#[derive(Serialize, Deserialize, Debug,  Clone, PartialEq)]
pub struct DictionaryGroup{
    pub id:Option<String>,
    pub name:String,
    pub alias:String
}