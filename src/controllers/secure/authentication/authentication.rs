use crate::controllers::secure::authentication::strategy::{AuthenticationError, Strategy};
use crate::controllers::secure::authentication::token::Token;
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository::Repository as User_repository;

pub struct Authentication {}

impl Authentication {
    pub async fn auth(token: &Token) -> bool {
        let x = Strategy::auth(&token).await;
        match x {
            Ok(user) => true,
            Err(err) => false
        }
    }
}