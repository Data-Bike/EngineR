use async_std::task::block_on;
use chrono::{DateTime, Utc};
use rocket::data::{FromData, ToByteUnit};
use rocket::{Data, Request};

use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest};

use serde_json::{from_str, Value};
use crate::controllers::form_parser::error::ParseError;
use crate::controllers::secure::authorization::token::Token;

use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::secure::entity::permission::{PermissionKind};
use crate::model::user;
use crate::model::user::entity::user::User;

const LIMIT: u32 = 1024 * 10;


pub fn getToken(req: &Request<'_>, object: &Object) -> Token {
    let requestKind = match req.method() {
        Method::Get => { PermissionKind::read }
        Method::Post => {
            match object.id {
                None => { PermissionKind::create }
                Some(_) => { PermissionKind::edit }
            }
        }
        _ => { PermissionKind::read }
    };

    let system = req.uri().path().segments().get(0).unwrap_or("").to_string();
    Token::fromObject(requestKind, system, object)
}

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



        let json_object_type: &Value = match json_object
            .get("filled")
        {
            None => { return Err(ParseError { message: format!("Error {} is not found", "filled") }); }
            Some(f) => { f }
        };

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

        let user_created = user::repository::repository::Repository::getUserById(user_created_id.to_string()).await?;

        let user_deleted_pre_res = user_deleted_id.and_then(|id| Some(block_on(user::repository::repository::Repository::getUserById(id))));
        let user_deleted = match user_deleted_pre_res {
            None => { None }
            Some(x) => { Some(x?) }
        };
        let date_created = DateTime::<Utc>::from(match DateTime::parse_from_rfc3339(date_str_created.as_str()) {
            Ok(d) => { d }
            Err(e) => { return Err(ParseError { message: "Error date_created is not rfc3339 date".to_string() }); }
        });
        let date_deleted = date_str_deleted
            .and_then(|d| DateTime::parse_from_rfc3339(d.as_str()).ok())
            .and_then(|d| Some(DateTime::<Utc>::from(d)));

        Ok(Object {
            filled: match
            ObjectType::from_json(json_object_type) {
                Ok(x) => { x }
                Err(e) => { return Err(e); }
            },
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
            Ok(v) => { v }
            Err(e) => { return Err(ParseError { message: "Error cannot parse json".to_string() }); }
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