use crate::controllers::secure::authorization::token::Token;
use crate::model::secure::entity::permission::Access::{allow, deny};
use crate::model::secure::entity::permission::PermissionLevel;
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
        let object = token.object.clone().unwrap();
        for group in &user.groups {
            for permission in &group.permissions.object {
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

pub struct ObjectTypeVote {}

impl ObjectTypeVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = token.object_type.clone().unwrap();
        for group in &user.groups {
            for permission in &group.permissions.object_type {
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

pub struct LinkVote {}

impl LinkVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = token.link.clone().unwrap();
        for group in &user.groups {
            for permission in &group.permissions.link {
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

pub struct LinkTypeVote {}

impl LinkTypeVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = token.link_type.clone().unwrap();
        for group in &user.groups {
            for permission in &group.permissions.link_type {
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

pub struct ObjectTypeFieldVote {}

impl ObjectTypeFieldVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        let object = token.object_type_field.clone().unwrap();
        for group in &user.groups {
            for permission in &group.permissions.object_type_field {
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
