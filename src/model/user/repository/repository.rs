use std::collections::LinkedList;
use std::time::SystemTime;
use async_std::task::{block_on, JoinHandle, spawn};
use chrono::{DateTime, NaiveDateTime, Offset, Utc};
use postgres::types::Timestamp;
use rocket::futures::future::err;
use rocket::time::PrimitiveDateTime;
use sqlx::Row;
use sqlx::Error as Sqlx_Error;
use crate::{cache_it, remove_it_from_cache};
use crate::controllers::pool::pool::{delete, insert, sql_one, update};
use crate::model::error::RepositoryError;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;
use crate::model::secure::repository::repository::Repository as Secure_Repository;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    pub async fn getUserById(id: String) -> Result<User, RepositoryError> {
        cache_it!(&id,user_by_login,{
            let groups = Secure_Repository::getUserGroupsbyUserId(id.clone()).await?;
            let row = sql_one(format!("select * from \"user\" where \"id\"='{}'", &id).as_str()).await?;
            User {
                id: Some(row.get::<String, &str>("login").to_string()),
                login: row.get::<String, &str>("login").to_string(),
                password: row.get::<String, &str>("password").to_string(),
                access_token: row.get::<Option<String>, &str>("access_token").unwrap_or("".to_string()).to_string(),
                oauth: row.get::<Option<String>, &str>("oauth").unwrap_or("".to_string()).to_string(),
                date_last_active: row.try_get::<NaiveDateTime, &str>("date_last_active").ok(),
                date_registred: row.get::<NaiveDateTime, &str>("date_registred"),
                groups: groups,
            }
        })
    }

    pub async fn getUserByLogin(login: String) -> Result<User, RepositoryError> {
        cache_it!(&login,user_by_login,{
            let row = sql_one(format!("select * from \"user\" where \"login\"='{}'", &login).as_str()).await?;
            User {
                id: Some(row.get::<String, &str>("login").to_string()),
                login: row.get::<String, &str>("login").to_string(),
                password: row.get::<String, &str>("password").to_string(),
                access_token: row.get::<Option<String>, &str>("access_token").unwrap_or("".to_string()).to_string(),
                oauth: row.get::<Option<String>, &str>("oauth").unwrap_or("".to_string()).to_string(),
                date_last_active: row.try_get::<NaiveDateTime, &str>("date_last_active").ok(),
                date_registred: row.get::<NaiveDateTime, &str>("date_registred"),
                groups: vec![],
            }
        })
    }


    pub fn userToNameValues(user: &User) -> Vec<(String, String)> {
        vec![
            ("login".to_string(), user.login.to_string()),
            ("password".to_string(), user.password.to_string()),
            ("access_token".to_string(), user.access_token.to_string()),
            ("date_last_active".to_string(), user
                .date_last_active
                .and_then(|d| Some(d.to_string()))
                .unwrap_or("".to_string())
            ),
            ("date_registred".to_string(), user.date_registred.to_string()),
        ]
    }

    pub async fn createUser(user: &User) -> Result<String, RepositoryError> {
        let nv_user = Self::userToNameValues(user);
        let id = insert("user".to_string(), nv_user).await?;
        let nv_user_groups = match user.groups
            .iter()
            .map(|g| g.id.as_ref())
            .collect::<Option<Vec<&String>>>()
        {
            None => { return Err(RepositoryError { message: format!("All groups must has id") }); }
            Some(g) => { g }
        }
            .iter()
            .map(|g| vec![
                ("user_id".to_string(), id.to_string()), ("group_id".to_string(), g.to_string()),
            ])
            .collect::<Vec<Vec<(String, String)>>>();
        let mut futures: Vec<JoinHandle<_>> = vec![];

        for nv_user_group in nv_user_groups {
            futures.push(
                spawn(
                    insert("r_user_group".to_string(), nv_user_group)
                )
            );
        }

        for future in futures {
            block_on(future)?;
        }

        Ok(id)
    }

    pub async fn updateUser(user: &User) -> Result<(), RepositoryError> {
        let id = match user.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("All groups must has id") }); }
            Some(i) => { i.to_string() }
        };
        remove_it_from_cache!(&id,user_by_id);
        remove_it_from_cache!(&user.login,user_by_login);

        let mut futures: Vec<JoinHandle<_>> = vec![];
        let exist_user = Self::getUserById(id.to_string()).await?;
        if exist_user.oauth != user.oauth
            || exist_user.access_token != user.access_token
            || exist_user.password != user.password
            || exist_user.login != user.login
        {
            let nv_user = Self::userToNameValues(user);
            futures.push(
                spawn(
                    update(
                        "user".to_string(),
                        nv_user,
                        vec![("id".to_string(), "=".to_string(), id.to_string())],
                    )
                )
            );
        }

        futures.push(
            spawn(
                delete(
                    "user_group".to_string(),
                    vec![],
                    vec![("user_id".to_string(), "=".to_string(), id.to_string())],
                )
            )
        );

        let nv_user_groups = match user.groups
            .iter()
            .map(|g| g.id.as_ref())
            .collect::<Option<Vec<&String>>>()
        {
            None => { return Err(RepositoryError { message: format!("All groups must has id") }); }
            Some(g) => { g }
        }
            .iter()
            .map(|g| vec![
                ("user_id".to_string(), id.to_string()), ("group_id".to_string(), g.to_string()),
            ])
            .collect::<Vec<Vec<(String, String)>>>();

        for nv_user_group in nv_user_groups {
            futures.push(
                spawn(
                    insert("user_group".to_string(), nv_user_group)
                )
            );
        }

        for future in futures {
            block_on(future)?;
        }

        Ok(())
    }


    pub async fn get_token_hashed_by_login(login: String) -> Result<String, RepositoryError> {
        let user = Self::getUserByLogin(login).await?;
        Ok(user.password)
    }
}