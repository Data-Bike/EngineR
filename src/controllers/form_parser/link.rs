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


impl Link {
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
        let object_from_id = match match json_object.get("object_from_id") {
            None => { return Err(ParseError { message: "Error object_from_id not found".to_string() }); }
            Some(v) => { v }
        }.as_str() {
            None => { return Err(ParseError { message: "Error object_from_id is not string".to_string() }); }
            Some(v) => { v }
        };
        let object_to_id = err_resolve!(json_object,"object_to_id");
        let user_created_id = err_resolve!(json_object,"user_created_id");
        let user_deleted_id = err_resolve!(json_object,"user_deleted_id");
        let date_created_str = err_resolve!(json_object,"date_created");
        let date_deleted_str = err_resolve!(json_object,"date_deleted");
        let object_from = Object_repository::hydrateFilledObjectType(object_from_id.to_string()).await;
        let object_to = Object_repository::hydrateFilledObjectType(object_to_id.to_string()).await;
        let user_created = User_repository::getUserById(user_created_id.to_string()).await;
        let user_deleted = if user_deleted_id == "" { None } else { Some(User_repository::getUserById(user_deleted_id.to_string()).await) };
        let date_created = DateTime::<Utc>::from(DateTime::parse_from_rfc3339(date_created_str).unwrap());
        let date_deleted = if date_deleted_str == "" { None } else { Some(DateTime::<Utc>::from(DateTime::parse_from_rfc3339(date_deleted_str).unwrap())) };


        Ok(Link {
            object_from,
            object_to,
            user_created,
            user_deleted,
            date_created,
            date_deleted,
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