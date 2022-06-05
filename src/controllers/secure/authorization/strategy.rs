use std::collections::BTreeMap;
use std::ops::Deref;
use rocket::form::validate::Contains;
// use serde_hjson::Map;
use crate::controllers::secure::authorization::token::Token;
use crate::controllers::secure::authorization::vote::{LinkTypeVote, LinkVote, ObjectTypeFieldVote, ObjectTypeVote, ObjectVote, SystemVote};
use crate::model::object::entity::object::ObjectType;
use crate::model::secure::entity::permission::PermissionLevel;
use crate::model::secure::entity::permission::PermissionLevel::link;
use crate::model::user::entity::user::User;


struct StrategyMap {
    kind: PermissionLevel,
    strategies: Vec<StrategyMap>,
}


pub struct Strategy {}

impl Strategy {
    pub fn new() -> Self {
        Self {}
    }


    pub fn resolve(user: &User, token: &Token) -> bool {
        if !SystemVote::allow(user, token) { return false; };

        if token.requestLevel == PermissionLevel::object {
            if !ObjectVote::allow(user, token) { return false; };
            if !ObjectTypeVote::allow(user, token) { return false; };

            match token.object_type.and_then(|ot| Some(ot.fields)) {
                None => { return false; }
                Some(fs) => {
                    for f in fs {
                        let mut t = Token::fromToken(token);
                        t.object_type_field = Some(f);
                        t.requestLevel = PermissionLevel::object_type_field;
                        if !ObjectTypeFieldVote::allow(user, &t) { return false; };
                    }
                }
            };
        }

        if token.requestLevel == PermissionLevel::object_type {
            if !ObjectTypeVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::object_type_field {
            if !ObjectTypeVote::allow(user, token) { return false; };
            if !ObjectTypeFieldVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::link {
            if !LinkVote::allow(user, token) { return false; };
            if !LinkTypeVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::link_type {
            if !LinkTypeVote::allow(user, token) { return false; };
        }
        true
    }
}