use crate::model::user::entity::user::User;
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Debug,Serialize, Deserialize,  Clone)]
pub enum Access {
    allow,
    deny,
}

#[derive(PartialEq, Debug,Serialize, Deserialize,  Clone)]
pub enum PermissionLevel {
    system,
    object,
    object_type,
    object_type_field,
    link,
    link_type,
}

#[derive(PartialEq, Debug,Serialize, Deserialize,  Clone)]
pub enum PermissionKind {
    create,
    read,
    edit,
}
#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct PermissionsGroup {
    pub system: Vec<Permission>,
    pub object: Vec<Permission>,
    pub object_type: Vec<Permission>,
    pub object_type_field: Vec<Permission>,
    pub link: Vec<Permission>,
    pub link_type: Vec<Permission>,
}
#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Group {
    pub alias: String,
    pub name: String,
    pub level: String,
    pub id: String,
    pub permissions: PermissionsGroup,
}
#[derive(Serialize, Deserialize, Debug,  Clone)]
pub struct Permission {
    pub access: Access,
    pub alias: String,
    pub id: String,
    pub level: PermissionLevel,
    pub kind: PermissionKind,
    pub name: String,
    pub object: String,
}