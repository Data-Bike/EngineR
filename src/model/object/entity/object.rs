use chrono::{DateTime, Utc};
use crate::model::user::entity::user::User;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Field {
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
    pub fields: Vec<Field>,
    pub kind: String,
    pub alias: String,
}

#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Object {
    pub(crate) filled: ObjectType,
    pub date_created: DateTime<Utc>,
    pub date_deleted: Option<DateTime<Utc>>,
    pub user_created: User,
    pub user_deleted: Option<User>,
    pub(crate) hash: String,
}