use chrono::{DateTime, NaiveDateTime, Utc};
use crate::model::secure::entity::permission::Group;
use serde::{Serialize, Deserialize, Serializer};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct User {
    pub id: Option<String>,
    pub login: String,
    #[serde(serialize_with = "User::ser_guard")]
    pub password: String,
    #[serde(serialize_with = "User::ser_guard")]
    pub access_token: String,
    #[serde(serialize_with = "User::ser_guard")]
    pub oauth: String,
    #[serde(serialize_with = "User::ser_groups_to_str")]
    pub groups: Vec<Group>,
    pub date_last_active: Option<NaiveDateTime>,
    pub date_registred: NaiveDateTime,
}

impl User {
    pub fn ser_groups_to_str<S: Serializer>(t: &Vec<Group>, s: S) -> Result<S::Ok, S::Error> {
        t
            .iter()
            .map(|g| g.id.clone())
            .collect::<Vec<Option<String>>>()
            .serialize(s)
    }
    pub fn ser_guard<S: Serializer>(t: &String, s: S) -> Result<S::Ok, S::Error> {
        "hidden".serialize(s)
    }
}