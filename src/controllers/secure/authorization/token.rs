use serde::de::Unexpected::Str;
use crate::controllers::secure::authorization::authorization::Authorization;
use crate::model::link::entity::link::{Link, LinkType};
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{PermissionKind, PermissionLevel};
use crate::model::user::entity::user::User;

pub struct Token {
    pub requestLevel: PermissionLevel,
    pub requestKind: PermissionKind,
    pub system: String,
    pub object_type: Option<ObjectType>,
    pub object_type_field: Option<Field>,
    pub object: Option<Object>,
    pub link_type: Option<LinkType>,
    pub link: Option<Link>,
    authorized: Option<bool>,
}

pub const  EmptyToken: Token = Token{
    requestLevel: PermissionLevel::system,
    requestKind: PermissionKind::create,
    system: String::new(),
    object_type: None,
    object_type_field: None,
    object: None,
    link_type: None,
    link: None,
    authorized: None
};


impl Token {
    pub fn fromObject(requestKind: PermissionKind, system: String, object: &Object) -> Token {
        Token {
            requestLevel: PermissionLevel::object,
            requestKind,
            system,
            object_type: Some(object.filled.clone()),
            object_type_field: None,
            object: Some(object.clone()),
            link_type: None,
            link: None,
            authorized: None,
        }
    }
    pub fn fromObjectType(requestKind: PermissionKind, system: String, object_type: &ObjectType) -> Token {
        Token {
            requestLevel: PermissionLevel::object_type,
            requestKind,
            system,
            object_type: Some(object_type.clone()),
            object_type_field: None,
            object: None,
            link_type: None,
            link: None,
            authorized: None,
        }
    }
    pub fn fromLink(requestKind: PermissionKind, system: String, link: &Link) -> Token {
        Token {
            requestLevel: PermissionLevel::link,
            requestKind,
            system,
            object_type: None,
            object_type_field: None,
            object: None,
            link_type: Some(link.link_type.clone()),
            link: Some(link.clone()),
            authorized: None,
        }
    }
    pub fn fromToken(token: &Token) -> Token {
        Token {
            requestLevel: token.requestLevel.clone(),
            requestKind: token.requestKind.clone(),
            system: token.system.clone(),
            object_type: token.object_type.clone(),
            object_type_field: token.object_type_field.clone(),
            object: token.object.clone(),
            link_type: token.link_type.clone(),
            link: token.link.clone(),
            authorized: None,
        }
    }


    pub fn is_authorized(&self) -> Option<bool> {
        self.authorized
    }

    pub fn approve(&mut self)  {
        self.authorized = Some(true);
    }

    pub fn decline(&mut self) {
        self.authorized = Some(false);
    }

    pub fn authorize(&mut self, user: &User) -> bool {
        let is_auth = Authorization::auth(user, self);
        self.authorized = Some(is_auth);
        return is_auth;
    }
}