use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use async_std::task::block_on;
use chrono::{DateTime, Utc};
use rocket::data::{FromData, ToByteUnit, Transform};
use rocket::{Data, Request, request};
use rocket::error::ErrorKind::Io;
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest, Outcome};
use serde::de::{Expected, Unexpected};
use serde_json::{from_str, Value};
use crate::controllers::secure::authorization::token::Token;
use crate::model::link::repository::repository::Repository;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{PermissionKind, PermissionLevel};
use crate::model::user;
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


pub fn getToken(req: &Request<'_>, object: &Object) -> Token {
    let requestKind = match req.method() {
        Method::Get => { PermissionKind::read }
        Method::Post => { PermissionKind::edit }
        _ => { panic!("Error") }
    };

    let system = req.get_param(0).unwrap().unwrap().to_string();
    Token::fromObject(requestKind, system, object)
}

impl Field {
    pub fn from_json(json: &Value) -> Result<Self, serde_json::Error> {
        Ok(Field {
            alias: json
                .get("alias")
                .unwrap_or(&Value::String("".to_string()))
                .as_str()
                .unwrap_or("")
                .to_string(),
            kind: json
                .get("kind")
                .unwrap_or(&Value::String("".to_string()))
                .as_str()
                .unwrap_or("")
                .to_string(),
            name: json
                .get("name")
                .unwrap_or(&Value::String("".to_string()))
                .as_str()
                .unwrap_or("")
                .to_string(),
            default: match json
                .get("default")
            {
                None => { None }
                Some(v) => {
                    match v.as_str() {
                        None => { None }
                        Some(v) => { Some(v.to_string()) }
                    }
                }
            },
            value: match json
                .get("value")
            {
                None => { None }
                Some(v) => {
                    match v.as_str() {
                        None => { None }
                        Some(v) => { Some(v.to_string()) }
                    }
                }
            },
            require: json
                .get("require")
                .unwrap_or(&Value::Bool(false))
                .as_bool()
                .unwrap_or(false),
            index: json
                .get("index")
                .unwrap_or(&Value::Bool(false))
                .as_bool()
                .unwrap_or(false),
            preview: json
                .get("preview")
                .unwrap_or(&Value::Bool(false))
                .as_bool()
                .unwrap_or(false),
        })
    }
}


impl Object {
    pub async fn from_json(json_object: &Value) -> Result<Self, serde_json::Error> {
        let json_object_type: &Value = json_object
            .get("filled")
            .unwrap();

        let user_created_id: &str = json_object
            .get("user_created")
            .unwrap()
            .as_str()
            .unwrap();

        let user_deleted_id: &str = json_object
            .get("user_deleted")
            .unwrap()
            .as_str()
            .unwrap();

        let date_str_created: &str = json_object
            .get("date_created")
            .unwrap()
            .as_str()
            .unwrap();

        let date_str_deleted: &str = json_object
            .get("date_deleted")
            .unwrap()
            .as_str()
            .unwrap();

        let user_created = user::repository::repository::Repository::getUserById(user_created_id.to_string()).await;
        let user_deleted = if user_deleted_id == "" { None } else { Some(user::repository::repository::Repository::getUserById(user_deleted_id.to_string()).await) };

        let date_created = DateTime::<Utc>::from(DateTime::parse_from_rfc3339(date_str_created).unwrap());
        let date_deleted = if date_str_deleted == "" { None } else { Some(DateTime::<Utc>::from(DateTime::parse_from_rfc3339(date_str_deleted).unwrap())) };

        Ok(Object {
            filled: ObjectType::from_json(json_object_type).unwrap(),
            date_created,
            date_deleted,
            user_created,
            user_deleted,
            hash: "".to_string(),
        })
    }

    pub async fn from_str(string: &str) -> Result<Self, serde_json::Error> {
        let json_object: Value = from_str(string)?;
        Self::from_json(&json_object).await
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Object {
    type Error = ParseError;
    type Owned = Data;
    type Borrowed = Data;

    fn transform(request: &Request, data: Data) -> Transform<rocket::data::Outcome<Self::Owned, Self::Error>> {
        Transform::Owned(Success(data))
    }


    async fn from_data(req: &'r Request<'_>, data: &mut Data) -> rocket::data::Outcome<Self, Self::Error> {
        let string = match data.open().into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, Self::Error { message: "Error".to_string() })),
            Err(e) => return Failure((Status::InternalServerError, Self::Error { message: "Error".to_string() })),
        };

        // We store `string` in request-local cache for long-lived borrows.
        //let string = request::local_cache!(req, string);

        let user = match User::from_request(req).await {
            Success(u) => {
                u
            }
            r => {
                r.and_then(|x| Failure((Status { code: 401, reason: "Error" }, ())))
            }
        };

        match Object::from_str(string.as_str()).await {
            Ok(o) => {
                if !getToken(req, &o).authorize(&user) {
                    Failure((Status { code: 403, reason: "Error" }, Self::Error { message: "Error".to_string() }))
                }
                Success(o)
            }
            Err(e) => { Failure((Status { code: 500, reason: "Error" }, Self::Error { message: "Error".to_string() })) }
        }
    }
}