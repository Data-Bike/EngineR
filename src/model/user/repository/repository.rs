use std::collections::LinkedList;
use rocket::futures::future::err;
use sqlx::Row;
use sqlx::Error as Sqlx_Error;
use crate::cache_it;
use crate::controllers::pool::pool::sql_one;
use crate::model::error::RepositoryError;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    pub async fn getUserById(id: String) -> Result<User, RepositoryError> {
        cache_it!(&id,user_by_login,{
            let row = sql_one(format!("select * from user where id={}", &id).as_str()).await?;
            User {
                id: Some(id),
                login: row.get::<String, &str>("login").to_string(),
                password: row.get::<String, &str>("password").to_string(),
                access_token: row.get::<String, &str>("access_token").to_string(),
                oauth: row.get::<String, &str>("oauth").to_string(),
                groups: vec![],
            }
        })
    }

    pub async fn getUserByLogin(login: String) -> Result<User, RepositoryError> {
        cache_it!(&login,user_by_login,{
            let row = sql_one(format!("select * from user where login={}", &login).as_str()).await?;
            User {
                id: Some(row.get::<String, &str>("login").to_string()),
                login: row.get::<String, &str>("login").to_string(),
                password: row.get::<String, &str>("password").to_string(),
                access_token: row.get::<String, &str>("access_token").to_string(),
                oauth: row.get::<String, &str>("oauth").to_string(),
                groups: vec![],
            }
        })
    }

    pub async fn get_token_hashed_by_login(login: String) -> Result<String, RepositoryError> {
        let user = Self::getUserByLogin(login).await?;
        Ok(user.password)
    }
}