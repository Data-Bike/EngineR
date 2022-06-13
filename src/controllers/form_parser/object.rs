use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use async_std::task::block_on;
use chrono::{DateTime, ParseResult, Utc};
use rocket::data::{FromData, ToByteUnit};
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
pub struct ParseError {
    pub message: String,
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

    let system = req.uri().path().segments().get(0).unwrap().to_string();
    Token::fromObject(requestKind, system, object)
}

// impl Field {
//     pub fn from_json(json: &Value) -> Result<Self, serde_json::Error> {
//         Ok(Field {
//             alias: json
//                 .get("alias")
//                 .unwrap_or(&Value::String("".to_string()))
//                 .as_str()
//                 .unwrap_or("")
//                 .to_string(),
//             kind: json
//                 .get("kind")
//                 .unwrap_or(&Value::String("".to_string()))
//                 .as_str()
//                 .unwrap_or("")
//                 .to_string(),
//             name: json
//                 .get("name")
//                 .unwrap_or(&Value::String("".to_string()))
//                 .as_str()
//                 .unwrap_or("")
//                 .to_string(),
//             default: match json
//                 .get("default")
//             {
//                 None => { None }
//                 Some(v) => {
//                     match v.as_str() {
//                         None => { None }
//                         Some(v) => { Some(v.to_string()) }
//                     }
//                 }
//             },
//             value: match json
//                 .get("value")
//             {
//                 None => { None }
//                 Some(v) => {
//                     match v.as_str() {
//                         None => { None }
//                         Some(v) => { Some(v.to_string()) }
//                     }
//                 }
//             },
//             require: json
//                 .get("require")
//                 .unwrap_or(&Value::Bool(false))
//                 .as_bool()
//                 .unwrap_or(false),
//             index: json
//                 .get("index")
//                 .unwrap_or(&Value::Bool(false))
//                 .as_bool()
//                 .unwrap_or(false),
//             preview: json
//                 .get("preview")
//                 .unwrap_or(&Value::Bool(false))
//                 .as_bool()
//                 .unwrap_or(false),
//         })
//     }
// }
//

impl Object {
    pub async fn from_json(json_object: &Value) -> Result<Self, ParseError> {
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



        let json_object_type: &Value = json_object
            .get("filled")
            .unwrap();

        let user_created_id = err_resolve!(json_object,"user_created").to_string();
        let user_deleted_id = err_resolve_option!(json_object,"user_deleted");


        let date_str_created = err_resolve!(json_object,"date_created").to_string();
        let date_str_deleted = err_resolve_option!(json_object,"date_deleted");


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

        let user_created = user::repository::repository::Repository::getUserById(user_created_id.to_string()).await;

        let user_deleted = user_deleted_id.and_then(|id| Some(block_on(user::repository::repository::Repository::getUserById(id))));

        let date_created = DateTime::<Utc>::from(match DateTime::parse_from_rfc3339(date_str_created.as_str()) {
            Ok(d) => { d }
            Err(e) => { return Err(ParseError { message: "Error date_created is not rfc3339 date".to_string() }); }
        });
        let date_deleted = date_str_deleted
            .and_then(|d| DateTime::parse_from_rfc3339(d.as_str()).ok())
            .and_then(|d| Some(DateTime::<Utc>::from(d)));

        Ok(Object {
            filled: ObjectType::from_json(json_object_type).unwrap(),
            date_created,
            date_deleted,
            user_created,
            user_deleted,
            hash: "".to_string(),
            id,
        })
    }

    pub async fn from_str(string: &str) -> Result<Self, ParseError> {
        let json_object: Value = match from_str::<Value>(string) {
            Ok(v) => {v}
            Err(e) => {return Err(ParseError { message: "Error cannot parse json".to_string() });}
        };
        Self::from_json(&json_object).await
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Object {
    type Error = ParseError;
    // type Owned = Data;
    // type Borrowed = Data;
    //
    // fn transform(request: &Request, data: Data) -> Transform<rocket::data::Outcome<Self::Owned, Self::Error>> {
    //     Transform::Owned(Success(data))
    // }


    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> rocket::data::Outcome<'r, Self, Self::Error> {
        let string = match data.open(LIMIT.bytes()).into_string().await {
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
                return Failure((Status { code: 401 }, Self::Error { message: "Error".to_string() }));
            }
        };

        match Object::from_str(string.as_str()).await {
            Ok(o) => {
                if !getToken(req, &o).authorize(&user) {
                    return Failure((Status { code: 403 }, Self::Error { message: "Error".to_string() }));
                }
                Success(o)
            }
            Err(e) => { Failure((Status { code: 500 }, Self::Error { message: "Error".to_string() })) }
        }
    }
}