use crate::controllers::secure::authorization::strategy::Strategy;
use crate::controllers::secure::authorization::token::Token;
use crate::model::user::entity::user::User;

pub struct Authorization {}

impl Authorization {
    pub fn auth(user: &User, token: &Token) -> bool {
        Strategy::resolve(user, token)
    }
}