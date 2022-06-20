use rocket::{Request};

use async_std::task::block_on;
use bcrypt::{BcryptResult, DEFAULT_COST};
use chrono::{DateTime, ParseResult, Utc};


use serde_json::{from_str, Value};


use crate::model::secure::entity::permission::Group;
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::secure::repository::repository::Repository as Secure_repository;
use crate::model::user::entity::user::User;

use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest};
use crate::controllers::form_parser::error::ParseError;

const LIMIT: u32 = 1024 * 10;


impl User {
    pub async fn from_str(string: &str) -> Result<Self, ParseError> {
        macro_rules! err_resolve {
            ( $x:expr, $key:expr ) => {
                match match $x.get($key) {
                    None => { return Err(ParseError { message: format!("Error {} not found",$key) }); }
                    Some(v) => { v }
                }.as_str() {
                    None => { return Err(ParseError { message: format!("Error {} is not string",$key) }); }
                    Some(v) => { v }
                }
            };
        }
        macro_rules! err_resolve_option {
            ( $x:expr, $key:expr ) => {
                match $x.get($key) {
                    None => { None }
                    Some(v) => {
                        match v.as_str() {
                            None => { return Err(ParseError { message: format!("Error {} is not string",$key) }); }
                            Some(v) => { Some(v.to_string()) }
                        }
                    }
                }
            };
        }

        let json_object: Value = match from_str::<Value>(string) {
            Ok(v) => { v }
            Err(e) => { return Err(ParseError { message: "Error cannot parse JSON".to_string() }); }
        };

        let login = err_resolve!(json_object,"login");
        let password = err_resolve!(json_object,"password");
        let access_token = err_resolve!(json_object,"access_token");
        let oauth = err_resolve!(json_object,"oauth");
        let date_registred_str = err_resolve!(json_object,"date_registred");
        let date_last_active_str = err_resolve_option!(json_object,"date_last_active");
        let date_registred = match DateTime::parse_from_rfc3339(date_registred_str) {
            Ok(d) => { DateTime::<Utc>::from(d) }
            Err(_) => { return Err(ParseError { message: format!("Cannt parse date") }); }
        }.naive_utc();
        let date_last_active = date_last_active_str
            .and_then(|d| DateTime::parse_from_rfc3339(d.as_str()).ok())
            .map(|d| DateTime::<Utc>::from(d).naive_utc());
        let groups = match match json_object.get("groups") {
            None => { return Err(ParseError { message: format!("Error {} not found", "groups") }); }
            Some(v) => { v }
        }.as_array() {
            None => { return Err(ParseError { message: format!("Error {} is not array", "groups") }); }
            Some(v) => { v }
        };

        let mut groups_s: Vec<Group> = vec![];

        for g in groups.iter()
        {
            match g.as_str() {
                None => { return Err(ParseError { message: format!("Error {} is not string", "group") }); }
                Some(g_id) => {
                    groups_s.push(Secure_repository::getGroupById(g_id).await?);
                }
            }
        };

        let id = match json_object.get("id") {
            None => { None }
            Some(v) => {
                match v.as_str()
                {
                    None => { return Err(ParseError { message: "Error id is not string".to_string() }); }
                    Some(v) => { Some(v.to_string()) }
                }
            }
        };


        Ok(User {
            id,
            login: login.to_string(),
            password: match bcrypt::hash(password.to_string(), DEFAULT_COST) {
                Ok(h) => { h }
                Err(e) => { return Err(ParseError { message: "Cannt hashed password".to_string() }); }
            },
            access_token: access_token.to_string(),
            oauth: oauth.to_string(),
            groups: groups_s,
            date_last_active,
            date_registred,
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ParseError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<User, ParseError> {
        request.cookies()
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse().ok())
            .map(|id| block_on(User_repository::getUserById(id)))
            .transpose()
            .ok()
            .flatten()
            .or_forward(())
    }
}
