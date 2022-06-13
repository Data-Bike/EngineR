use std::fmt::{Debug, Display, Formatter};
use rocket::data::{FromData, Outcome, ToByteUnit};
use rocket::{Data, Request};
use std::error::Error;
use async_std::task::block_on;
use chrono::{DateTime, Utc};
use sqlx::Error as Sqlx_Error;
use rocket::http::Status;
use rocket::outcome::Outcome::{Failure, Success};
use serde_json::{from_str, json, Value};
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository as Object_repository;
use crate::model::secure::entity::permission::Group;
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::secure::repository::repository::Repository as Secure_repository;
use crate::model::user::entity::user::User;

use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest};

const LIMIT: u32 = 1024 * 10;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ParseError {}


impl From<Sqlx_Error> for ParseError {
    fn from(e: Sqlx_Error) -> Self {
        let message = match e {
            Sqlx_Error::Configuration(e) => {
                e.to_string()
            }
            Sqlx_Error::Database(e) => { format!("Error returned from the database: '{}'", e.message()) }
            Sqlx_Error::Io(e) => { format!("Error communicating with the database backend: '{}'", e) }
            Sqlx_Error::Tls(e) => { format!("Error occurred while attempting to establish a TLS connection: '{}'", e) }
            Sqlx_Error::Protocol(e) => { format!("Unexpected or invalid data encountered while communicating with the database(Driver may be corrupted): '{}'", e) }
            Sqlx_Error::RowNotFound => { format!("No rows returned by a query that expected to return at least one row") }
            Sqlx_Error::TypeNotFound { type_name } => { format!("Type '{}' Not Found", type_name) }
            Sqlx_Error::ColumnIndexOutOfBounds { index, len } => { format!("Column index out of bounds: the len is {}, but the index is {}", len, index) }
            Sqlx_Error::ColumnNotFound(e) => { format!("No column found for the given name: '{}'", e) }
            Sqlx_Error::ColumnDecode { index, source } => { format!("Error occurred while decoding column {}: {}", index, source) }
            Sqlx_Error::Decode(e) => { format!("Error occurred while decoding a value: '{}'", e) }
            Sqlx_Error::PoolTimedOut => { format!("Pool Timed Out Error") }
            Sqlx_Error::PoolClosed => { format!("Pool Closed Error") }
            Sqlx_Error::WorkerCrashed => { format!("Worker Crashed Error") }
            Sqlx_Error::Migrate(e) => { format!("Migrate Error") }
            _ => { format!("Unknown SQLX DB ERROR") }
        };

        Self { message }
    }
}

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

        let json_object: Value = match from_str::<Value>(string) {
            Ok(v) => { v }
            Err(e) => { return Err(ParseError { message: "Error cannot parse JSON".to_string() }); }
        };

        let login = err_resolve!(json_object,"login");
        let password = err_resolve!(json_object,"password");
        let access_token = err_resolve!(json_object,"access_token");
        let oauth = err_resolve!(json_object,"oauth");
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
            password: password.to_string(),
            access_token: access_token.to_string(),
            oauth: oauth.to_string(),
            groups: groups_s,
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
