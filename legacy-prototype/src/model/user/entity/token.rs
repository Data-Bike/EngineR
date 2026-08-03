use serde::ser::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Token {
    pub id: String,
    pub token_hashed: String,
}
