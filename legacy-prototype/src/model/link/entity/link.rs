use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::user::entity::user::User;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkType {
    pub id: Option<String>,
    pub alias: String,
    pub name: String,
    pub object_type_from: ObjectType,
    pub object_type_to: ObjectType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub id: Option<String>,
    pub object_from: Object,
    pub object_to: Object,
    pub link_type: LinkType,
    pub user_created: User,
    pub user_deleted: Option<User>,
    pub date_created: NaiveDateTime,
    pub date_deleted: Option<NaiveDateTime>,
}