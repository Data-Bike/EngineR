use chrono::{DateTime, Utc};
use crate::model::user::entity::user::User;

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

pub struct ObjectType {
    pub fields: Vec<Field>,
    pub kind: String,
    pub alias: String,
}

pub struct Object {
    pub(crate) filled: ObjectType,
    pub date_created: DateTime<Utc>,
    pub date_deleted: Option<DateTime<Utc>>,
    pub user_created: User,
    pub user_deleted: Option<User>,
    pub(crate) hash: String,
}