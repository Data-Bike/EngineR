use crate::controllers::secure::authorization::token::Token;

use crate::model::secure::entity::permission::Access::{allow, deny};

use crate::model::user::entity::user::User;

pub struct SystemVote {}

impl SystemVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        println!("SystemVote {:?}", token);
        let object = token.system.clone();
        for group in &user.groups {
            for permission in &group.permissions.system {
                println!("SystemVote permission {:?}", permission);
                println!("SystemVote object {:?}", object);
                println!("SystemVote requestKind {:?}", token.requestKind);
                if permission.object == object &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    println!("SystemVote requestKind approve");
                    return true;
                }
            }
        }
        println!("SystemVote requestKind denied");
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
                    permission.access == deny {
                    return false;
                }
            }
        }
        return true;
    }
}

pub struct ObjectTypeVote {}

impl ObjectTypeVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        println!("Start ObjectTypeVote::allow...");
        let object = match token.object_type.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        println!("Start ObjectTypeVote::allow ok");
        for group in &user.groups {
            println!("Start ObjectTypeVote::allow group '{}'...", group.alias);
            for permission in &group.permissions.object_type {
                println!("Start ObjectTypeVote::allow permission '{}'...", permission.alias);
                if permission.object == *match object.id.as_ref() {
                    None => {
                        println!("Start ObjectTypeVote::allow permission NO OBJECT ID '{}'", permission.alias);
                        return true;
                    }
                    Some(o) => { o }
                } &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    return true;
                }
                println!("Start ObjectTypeVote::allow permission '{}' ok", permission.alias);
            }
            println!("Start ObjectTypeVote::allow group '{}' ok", group.alias);
        }
        return false;
    }
}

pub struct LinkVote {}

impl LinkVote {
    pub fn allow(user: &User, token: &Token) -> bool {
        println!("Start LinkVote::allow get object...");
        let object = match token.link.as_ref() {
            None => { return false; }
            Some(o) => { o }
        };
        println!("Start LinkVote::allow object ok");
        println!("Start LinkVote::allow check permissions...");
        for group in &user.groups {
            for permission in &group.permissions.link {
                if permission.object == *match object.id.as_ref() {
                    None => {
                        println!("Start LinkVote::allow denied no object id");
                        return false;
                    }
                    Some(o) => { o }
                } &&
                    permission.kind == token.requestKind &&
                    permission.access == allow {
                    println!("Start LinkVote::allow approve by *");
                    return true;
                }
            }
        }
        println!("Start LinkVote::allow approve by no permissions {:?}",user.groups);
        return true;
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
                if (Some(permission.object.clone()) == object.id  || permission.object == "*".to_string())  &&
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
        let object = match token.object.as_ref().and_then(|f| Some(f.filled.fields.clone())) {
            None => { return false; }
            Some(o) => { o }
        };
        for field in object.iter() {
            let mut f_object = match field.id.as_ref() {
                None => { return false; }
                Some(f) => { f.to_string() }
            };
            let mut is_a = false;
            for group in &user.groups {
                println!("Test ObjectTypeFieldVote f_object {}", f_object);
                for permission in &group.permissions.object_type_field {
                    println!("Test ObjectTypeFieldVote {}=={}", permission.object, f_object);
                    if permission.object == f_object &&
                        permission.kind == token.requestKind &&
                        permission.access == allow {
                        is_a = true;
                    }
                }
            };
            if !is_a {
                return false;
            }
        }
        return true;
    }
}
