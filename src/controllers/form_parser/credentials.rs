use std::fmt::{Debug, Display, Formatter};
use rocket::data::{FromData, Outcome, ToByteUnit, Transform};
use rocket::{Data, Request};
use std::error::Error;
use std::net::IpAddr;
use std::net::IpAddr::{V4, V6};
use chrono::{DateTime, Utc};
use rocket::http::Method::Post;
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::FromRequest;
use serde_json::{from_str, json, Value};
use crate::controllers::secure::authentication::credentials::CheckCredentials::{AccessToken, Password};
use crate::controllers::secure::authentication::credentials::Credentials;
use crate::controllers::secure::authentication::token::IP::{v4, v6};
use crate::controllers::secure::authentication::token::Token;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository as Object_repository;
use crate::model::secure::entity::permission::{PermissionKind, PermissionLevel};
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::link::repository::repository::Repository as Link_repository;
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


impl Credentials {
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
        let login = err_resolve!(json_object,"login").to_string();

        let checkCredentials = match json_object.get("password") {
            None => {
                match json_object.get("access_token") {
                    None => {
                        return Err(ParseError { message: format!("Error must be 'password' or 'access_token'") });
                    }
                    Some(v) => {
                        AccessToken(match v.as_str() {
                            None => { return Err(ParseError { message: format!("Error {} is not string", "access_token") }); }
                            Some(v) => { v }
                        }.to_string())
                    }
                }
            }
            Some(v) => {
                Password(match v.as_str() {
                    None => { return Err(ParseError { message: format!("Error {} is not string", "password") }); }
                    Some(v) => { v }
                }.to_string())
            }
        };

        Ok(Credentials {
            login,
            checkCredentials,
        })
    }


    pub async fn from_json(json_object: &Value) -> Result<Self, ParseError> {
        let login = err_resolve!(json_object,"login").to_string();

        let checkCredentials = match json_object.get("password") {
            None => {
                match json_object.get("access_token") {
                    None => {
                        return Err(ParseError { message: format!("Error must be 'password' or 'access_token'") });
                    }
                    Some(v) => {
                        AccessToken(match v.as_str() {
                            None => { return Err(ParseError { message: format!("Error {} is not string", "access_token") }); }
                            Some(v) => { v }
                        }.to_string())
                    }
                }
            }
            Some(v) => {
                Password(match v.as_str() {
                    None => { return Err(ParseError { message: format!("Error {} is not string", "password") }); }
                    Some(v) => { v }
                }.to_string())
            }
        };

        Ok(Credentials {
            login,
            checkCredentials,
        })
    }
}

impl Token {
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
        let credentials = match Credentials::from_json(&json_object).await {
            Ok(c) => { c }
            Err(e) => { return Err(ParseError { message: "Error cannot parse Credentials".to_string() }); }
        };

        Ok(Token::new(credentials, ip))
    }
}


#[rocket::async_trait]
impl<'r> FromData<'r> for Credentials {
    type Error = ParseError;
    type Owned = Data;
    type Borrowed = Data;

    fn transform(request: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        Transform::Owned(Success(data))
    }

    async fn from_data(req: &'r Request<'_>, data: &mut Data) -> Outcome<Self, Self::Error> {
        let string = match data.open().into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, Self::Error { message: "Error".to_string() })),
            Err(e) => return Failure((Status::InternalServerError, Self::Error { message: "Error".to_string() })),
        };

        match Credentials::from_str(string.as_str()).await {
            Ok(o) => {
                Success(o)
            }
            Err(e) => { Failure((Status { code: 500, reason: "Error" }, Self::Error { message: "Error".to_string() })) }
        }
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Token {
    type Error = ParseError;
    type Owned = Data;
    type Borrowed = Data;

    fn transform(request: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        Transform::Owned(Success(data))
    }

    async fn from_data(req: &'r Request<'_>, data: &mut Data) -> Outcome<Self, Self::Error> {
        let ip = match req.client_ip() {
            Some(i) => {
                match i {
                    V4(ipv4) => {
                        v4(ipv4.to_string())
                    }
                    V6(ipv6) => {
                        v6(ipv6.to_string())
                    }
                }
            }
            None => { return Failure((Status::InternalServerError, Self::Error { message: "Error no ip".to_string() })); }
        };
        let string = match data.open().into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, Self::Error { message: "Error".to_string() })),
            Err(e) => return Failure((Status::InternalServerError, Self::Error { message: "Error".to_string() })),
        };

        let credentials = match Credentials::from_str(string.as_str()).await {
            Ok(o) => {
                o
            }
            Err(e) => { return Failure((Status { code: 500, reason: "Error" }, Self::Error { message: "Error".to_string() })); }
        };

        let mut dirty_token = Token::new(credentials, ip);
        dirty_token.authenticate();

        Success(dirty_token)
    }
}
