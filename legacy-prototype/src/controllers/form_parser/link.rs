use rocket::data::{FromData, Outcome, ToByteUnit};
use rocket::{Data, Request};


use chrono::{DateTime, Utc};
use futures::executor::block_on;

use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::FromRequest;
use serde_json::{from_str, Value};
use crate::controllers::form_parser::error::ParseError;
use crate::controllers::secure::authorization::token::Token;
use crate::model::link::entity::link::Link;

use crate::model::object::repository::repository::Repository as Object_repository;
use crate::model::secure::entity::permission::{PermissionKind};
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::link::repository::repository::Repository as Link_repository;
use crate::model::user::entity::user::User;
use crate::user;

const LIMIT: u32 = 1024 * 10;


pub fn getToken(req: &Request<'_>, link: &Link) -> Token {
    let requestKind = match req.method() {
        Method::Get => { PermissionKind::read }
        Method::Post => {
            match link.id {
                None => { PermissionKind::create }
                Some(_) => { PermissionKind::edit }
            }
        }
        _ => { PermissionKind::read }
    };

    let system = req.uri().path().segments().get(0).unwrap_or("").to_string();
    Token::fromLink(requestKind, system, link)
}

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
        let object_from_id = match match json_object.get("object_from_id") {
            None => { return Err(ParseError { message: "Error object_from_id not found".to_string() }); }
            Some(v) => { v }
        }.as_str() {
            None => { return Err(ParseError { message: "Error object_from_id is not string".to_string() }); }
            Some(v) => { v }
        };
        let object_to_id = err_resolve!(json_object,"object_to_id");
        let user_created_id = err_resolve!(json_object,"user_created_id");
        let user_deleted_id = err_resolve_option!(json_object,"user_deleted_id");
        let link_type_id = err_resolve!(json_object,"link_type_id");
        let date_created_str = err_resolve!(json_object,"date_created");
        let date_deleted_str = err_resolve_option!(json_object,"date_deleted");

        let user_deleted_pre_res = user_deleted_id.and_then(|id| Some(block_on(User_repository::getUserById(id))));
        let user_deleted = match user_deleted_pre_res {
            None => { None }
            Some(x) => { Some(x?) }
        };


        let date_deleted = date_deleted_str
            .and_then(|d| DateTime::parse_from_rfc3339(d.as_str()).ok())
            .and_then(|d| Some(DateTime::<Utc>::from(d).naive_utc()));


        let date_created = DateTime::<Utc>::from(match DateTime::parse_from_rfc3339(date_created_str) {
            Ok(d) => { d }
            Err(e) => { return Err(ParseError { message: "Error date_created is not rfc3339 date".to_string() }); }
        }).naive_utc();


        let object_from = Object_repository::hydrateFilledObjectType(object_from_id.to_string()).await?;
        let object_to = Object_repository::hydrateFilledObjectType(object_to_id.to_string()).await?;
        let user_created = User_repository::getUserById(user_created_id.to_string()).await?;

        let link_type = Link_repository::getLinkTypeById(link_type_id).await?;
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

        Ok(Link {
            id,
            object_from,
            object_to,
            link_type,
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
    // type Owned = Data;
    // type Borrowed = Data;
    //
    // fn transform(request: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
    //     Transform::Owned(Success(data))
    // }

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self, Self::Error> {
        let string = match data.open(LIMIT.bytes()).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(r) => return Failure((Status::PayloadTooLarge, Self::Error { message: format!("Error {}",r.value)  })),
            Err(e) => return Failure((Status::InternalServerError, Self::Error { message: format!("Error {}",e)  })),
        };
        let user = match User::from_request(req).await {
            Success(u) => {
                u
            }
            r => {
                return Failure((Status { code: 401 }, Self::Error { message: format!("Error {}",r.to_string()) }));
            }
        };
        match Link::from_str(string.as_str()).await {
            Ok(o) => {
                if !getToken(req, &o).authorize(&user) {
                    return Failure((Status { code: 403 }, Self::Error { message: format!("Error {}","authorize") }));
                }
                Success(o)
            }
            Err(e) => { Failure((Status { code: 500 }, Self::Error { message: format!("Error {}",e) })) }
        }
    }
}
