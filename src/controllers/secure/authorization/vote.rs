use crate::controllers::secure::authorization::token::Token;

use crate::model::secure::entity::permission::Access::{allow};

use crate::model::user::entity::user::User;

pub struct SystemVote {}

impl SystemVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = token.system.clone();
        for group in &user.groups {
            for permission in &group.permissions.system {
                if permission.object == object &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}

pub struct ObjectVote {}

impl ObjectVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = match token.object.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        for group in &user.groups {
            for permission in &group.permissions.object {
                if permission.object == *match object.id.as_ref() {
                    None => { return false; }
                    Some(o) => { o }
                } &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}

pub struct ObjectTypeVote {}

impl ObjectTypeVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = match token.object_type.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        for group in &user.groups {
            for permission in &group.permissions.object_type {
                if permission.object == *match object.id.as_ref() {
                    None => { return false; }
                    Some(o) => { o }
                } &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}

pub struct LinkVote {}

impl LinkVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = match token.link.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        for group in &user.groups {
            for permission in &group.permissions.link {
                if permission.object == *match object.id.as_ref() {
                    None => { return false; }
                    Some(o) => { o }
                } &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}

pub struct LinkTypeVote {}

impl LinkTypeVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = match token.link_type.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        for group in &user.groups {
            for permission in &group.permissions.link_type {
                if Some(permission.object.clone()) == object.id &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}

pub struct ObjectTypeFieldVote {}

impl ObjectTypeFieldVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = match token.object_type_field.as_ref().and_then(|f| Some(f.alias.clone())) {
            None => { return false; }
            Some(o) => { o }
        };
        for group in &user.groups {
            for permission in &group.permissions.object_type_field {
                if permission.object == *object &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
            }
        }
        return false;
    }
}
