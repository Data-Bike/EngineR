use rocket::{Data, Request};

use async_std::task::block_on;
use bcrypt::{BcryptResult, DEFAULT_COST};
use chrono::{DateTime, NaiveDateTime, ParseResult, Utc};
use rocket::data::{FromData, ToByteUnit};
use rocket::http::{Method, Status};


use serde_json::{from_str, Value};


use crate::controllers::secure::authorization::token::Token;
use crate::model::secure::entity::permission::{Group, PermissionKind};
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::secure::repository::repository::Repository as Secure_repository;
use crate::model::user::entity::user::User;

use rocket::outcome::IntoOutcome;
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{self, FromRequest};
use crate::controllers::form_parser::error::ParseError;
use crate::model::error::RepositoryError;

const LIMIT: u32 = 1024 * 10;


pub fn getToken(req: &Request<'_>, object: &User) -> Token {
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
    Token::fromUser(requestKind, system)
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
        macro_rules! err_resolve_option {
            ( $x:expr, $key:expr ) => {
                match $x.get($key) {
                    None => { None }
                    Some(v) => {
                        match v.as_str() {
                            None => { None }
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
        let date_registred = match NaiveDateTime::parse_from_str(date_registred_str,"%Y-%m-%dT%H:%M:%S%.6f") {
            Ok(d) => {d}
            Err(e) => { return Err(ParseError { message: format!("Cannt parse date with value:'{}' {}",date_registred_str,e) }); }
        };
        let date_last_active = date_last_active_str
            .and_then(|d| NaiveDateTime::parse_from_str(d.as_str(),"%Y-%m-%dT%H:%M:%S%.6f").ok());
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
        for c in request.cookies().iter() {
            println!("C:{:?} {:?} {:?}", c.name(), c.value(), c.secure(), )
        }
        println!("Request start to parse user: {}", request.cookies().get_private("user_id").map(|c| c.value().to_string()).unwrap_or("User not found in the cookie".to_string()));
        match request.cookies()
            .get_private("user_id")
            .and_then(|cookie| {
                println!("Cookie to parse user: {}", cookie.value());
                cookie.value().parse().ok()
            })
            .map(|id| {
                println!("User id from cookie: {}", id);
                block_on(User_repository::getUserById(id))
            })
            .transpose() {
            Ok(u) => { u }
            Err(e) => { return Failure((Status { code: 403 }, ParseError { message: format!("Error get user from cookie: {:?}", e) })); }
        }
            .or_forward(())
    }
}


#[rocket::async_trait]
impl<'r> FromData<'r> for User {
    type Error = ParseError;

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
                return Failure((Status { code: 401 }, Self::Error {
                    message: format!("Error {:?}", r)
                }));
            }
        };

        match User::from_str(string.as_str()).await {
            Ok(o) => {
                if !getToken(req, &o).authorize(&user) {
                    return Failure((Status { code: 403 }, Self::Error { message: "Error authorize token".to_string() }));
                }
                Success(o)
            }
            Err(e) => { Failure((Status { code: 500 }, Self::Error { message: format!("Error other {:?}", e.message) })) }
        }
    }
}
