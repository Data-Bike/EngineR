
use crate::model::secure::entity::permission::Group;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct User {
    pub id: Option<String>,
    pub login: String,
    pub password: String,
    pub access_token: String,
    pub oauth: String,
    pub groups:Vec<Group>
}