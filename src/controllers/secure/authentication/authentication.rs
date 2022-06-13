use crate::controllers::secure::authentication::strategy::{Strategy};
use crate::controllers::secure::authentication::token::Token;


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