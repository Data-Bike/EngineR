use crate::model::link::entity::link::{Link, LinkType};
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{PermissionKind, PermissionLevel};

pub struct Token {
    pub requestLevel: PermissionLevel,
    pub requestKind: PermissionKind,
    pub system: String,
    pub object_type: Option<ObjectType>,
    pub object_type_field: Option<Field>,
    pub object: Option<Object>,
    pub link_type: Option<LinkType>,
    pub link: Option<Link>,
    pub authorized: Option<bool>,
}

impl Token {
    pub fn is_authorized(&self) -> Option<bool> {
        self.authorized
    }

    pub fn approve(&mut self) -> Option<bool> {
        self.authorized = Some(true);
    }

    pub fn decline(&mut self) {
        self.authorized = Some(false);
    }
}