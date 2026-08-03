use crate::controllers::secure::authorization::token::Token;
use crate::controllers::secure::authorization::vote::{LinkTypeVote, LinkVote, ObjectTypeFieldVote, ObjectTypeVote, ObjectVote, SystemVote};

use crate::model::secure::entity::permission::PermissionLevel;

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
        println!("Start SystemVote::allow");
        if !SystemVote::allow(user, token) { return false; };

        if token.requestLevel == PermissionLevel::object {
            println!("Start ObjectVote::allow");
            if !ObjectVote::allow(user, token) { return false; };
            println!("Start ObjectTypeVote::allow");
            if !ObjectTypeVote::allow(user, token) { return false; };

            match token.object_type.clone().and_then(|ot| Some(ot.fields)) {
                None => { return false; }
                Some(fs) => {
                    for f in fs {
                        let mut t = Token::fromToken(token);
                        t.object_type_field = Some(f.clone());
                        t.requestLevel = PermissionLevel::object_type_field;
                        println!("Start ObjectTypeFieldVote::allow");
                        if !ObjectTypeFieldVote::allow(user, &t) { return false; };
                    }
                }
            };
        }

        if token.requestLevel == PermissionLevel::object_type {
            println!("Start ObjectTypeVote::allow");
            if !ObjectTypeVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::object_type_field {
            println!("Start ObjectTypeVote::allow");
            if !ObjectTypeVote::allow(user, token) { return false; };
            println!("Start ObjectTypeFieldVote::allow");
            if !ObjectTypeFieldVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::link {
            println!("Start LinkVote::allow");
            if !LinkVote::allow(user, token) { return false; };
            println!("Start LinkTypeVote::allow");
            if !LinkTypeVote::allow(user, token) { return false; };
        }

        if token.requestLevel == PermissionLevel::link_type {
            println!("Start LinkTypeVote::allow");
            if !LinkTypeVote::allow(user, token) { return false; };
        }
        true
    }
}