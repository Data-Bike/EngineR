use std::fmt::{Debug, Display, Formatter};
use rocket::data::{FromData, Outcome, ToByteUnit};
use rocket::{Data, Request};
use std::error::Error;
use chrono::{DateTime, Utc};
use rocket::http::Status;
use rocket::outcome::Outcome::{Failure, Success};
use serde_json::{from_str, json, Value};
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository as Object_repository;
use crate::model::secure::entity::permission::Group;
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::user::entity::user::User;

const LIMIT: u32 = 1024 * 10;

#[derive(Debug)]
struct ParseError {
    message: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ParseError {}

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
        let groups = match match json_object.get("groups") {
            None => { return Err(ParseError { message: format!("Error {} not found","groups") }); }
            Some(v) => { v }
        }.as_array() {
            None => { return Err(ParseError { message: format!("Error {} is not array","groups") }); }
            Some(v) => { v }
        };
        groups.iter().map(|g| match g.as_str() {
            None => { return Err(ParseError { message: format!("Error {} is not array","groups") }); }
            Some(g_id) => {}
        } )

        Ok(User{
            id: "",
            login,
            password,
            groups: vec![]
        })
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Link {
    type Error = ParseError;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let string = match data.open(LIMIT.bytes()).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, Self::Error { message: "Error".to_string() })),
            Err(e) => return Failure((Status::InternalServerError, Self::Error { message: "Error".to_string() })),
        };
        match Link::from_str(string.as_str()).await {
            Ok(o) => { Success(o) }
            Err(e) => { Failure((Status { code: 500 }, Self::Error { message: "Error".to_string() })) }
        }
    }
}
