use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Access {
    allow,
    deny,
}

impl ToString for Access {
    fn to_string(&self) -> String {
        match self {
            Access::allow => { "allow".to_string() }
            Access::deny => { "deny".to_string() }
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum PermissionLevel {
    system,
    object,
    object_type,
    object_type_field,
    link,
    link_type,
}


impl ToString for PermissionLevel {
    fn to_string(&self) -> String {
        match self {
            PermissionLevel::system => { "system".to_string() }
            PermissionLevel::object => { "object".to_string() }
            PermissionLevel::object_type => { "object_type".to_string() }
            PermissionLevel::object_type_field => { "object_type_field".to_string() }
            PermissionLevel::link => { "link".to_string() }
            PermissionLevel::link_type => { "link_type".to_string() }
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum PermissionKind {
    create,
    read,
    edit,
}

impl ToString for PermissionKind {
    fn to_string(&self) -> String {
        match self {
            PermissionKind::create => { "create".to_string() }
            PermissionKind::read => { "read".to_string() }
            PermissionKind::edit => { "edit".to_string() }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PermissionsGroup {
    pub system: Vec<Permission>,
    pub object: Vec<Permission>,
    pub object_type: Vec<Permission>,
    pub object_type_field: Vec<Permission>,
    pub link: Vec<Permission>,
    pub link_type: Vec<Permission>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Group {
    pub alias: String,
    pub name: String,
    pub level: String,
    pub id: Option<String>,
    pub permissions: PermissionsGroup,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Permission {
    pub access: Access,
    pub alias: String,
    pub id: Option<String>,
    pub level: PermissionLevel,
    pub kind: PermissionKind,
    pub name: String,
    pub object: String,
}