use crate::controllers::secure::authentication::strategy::{AuthenticationError, Strategy};
use crate::controllers::secure::authentication::token::Token;
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository::Repository;

pub struct Authentication{

}
impl Authentication{
    pub async fn auth(token:Token,user_model:Repository)->bool{
        let x =Strategy{user_model}.auth(&token).await;
        match x {
            Ok(user) => true,
            Err(err) => false
        }
    }
}