use chrono::{DateTime, NaiveDateTime, Utc};
use crate::model::user::entity::user::User;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Field {
    pub id: Option<String>,
    pub alias: String,
    pub kind: String,
    pub name: String,
    pub default: Option<String>,
    pub value: Option<String>,
    pub require: bool,
    pub index: bool,
    pub preview: bool,
}

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct ObjectType {
    pub id: Option<String>,
    pub fields: Vec<Field>,
    pub kind: String,
    pub alias: String,
}

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Object {
    pub(crate) filled: ObjectType,
    pub date_created: NaiveDateTime,
    pub date_deleted: Option<NaiveDateTime>,
    pub user_created: User,
    pub user_deleted: Option<User>,
    pub(crate) hash: String,
    pub(crate) id: Option<String>,
}