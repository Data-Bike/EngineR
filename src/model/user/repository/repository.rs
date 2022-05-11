use std::collections::LinkedList;
use rocket::futures::future::err;
use sqlx::Row;
use crate::controllers::pool::pool::sql_one;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    pub async fn getUserById(&mut self, id: String) -> User {
        let row = sql_one(format!("select * from user where id={}", &id).as_str()).await;
        User {
            id,
            login: row.get::<String, &str>("login").to_string(),
            token_hashed: row.get::<String, &str>("token_hashed").to_string(),
            groups: vec![],
        }
    }

    pub async fn getUserByLogin(&mut self, login: String) -> User {
        let row = sql_one(format!("select * from user where login={}", &login).as_str()).await;
        User {
            id: "".to_string(),
            login: row.get::<String, &str>("login").to_string(),
            token_hashed: row.get::<String, &str>("token_hashed").to_string(),
            groups: vec![],
        }
    }

    pub async fn get_token_hashed_by_login(&mut self, login: String) -> String {
        let user = self.getUserByLogin(login).await;
        user.token_hashed
    }
}