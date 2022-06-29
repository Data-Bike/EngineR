use rocket::data::{FromData, ToByteUnit};
use rocket::{Data, Request};

use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest};


use serde_json::{from_str, Value};
use crate::controllers::form_parser::error::ParseError;
use crate::controllers::secure::authorization::token::{Token};
use crate::model::dictionary::entity::dictionary::{Dictionary, DictionaryType};
use crate::model::dictionary::repository::repository::Repository;

use crate::model::secure::entity::permission::{PermissionKind};

use crate::model::user::entity::user::User;

const LIMIT: u32 = 1024 * 10;


pub fn getToken(req: &Request<'_>, dictionary_type: &DictionaryType) -> Token {
    let requestKind = match req.method() {
        Method::Get => { PermissionKind::read }
        Method::Post => {
            match dictionary_type.id {
                None => { PermissionKind::create }
                Some(_) => { PermissionKind::edit }
            }
        }
        _ => { PermissionKind::read }
    };

    let system = req.uri().path().segments().get(0).unwrap_or("").to_string();
    Token::fromDictionaryType(requestKind, system, dictionary_type)
}


impl DictionaryType {
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

        let mut dictionaries:Vec<Dictionary> = vec![];

        let id = match json_object.get("id") {
            None => { None }
            Some(v) => {
                match v.as_str()
                {
                    None => { return Err(ParseError { message: "Error id is not string".to_string() }); }
                    Some(v) => {
                        dictionaries = Repository::getDictionariesByTypeId(v.to_string()).await?;
                        Some(v.to_string())
                    }
                }
            }
        };

        let name = err_resolve!(json_object,"name").to_string();
        let alias = err_resolve!(json_object,"alias").to_string();

        Ok(DictionaryType {
            id,
            dictionaries,
            name,
            alias,
        })
    }
    pub async fn from_str(string: &str) -> Result<Self, ParseError> {
        let json_object: Value = match from_str::<Value>(string) {
            Ok(v) => { v }
            Err(e) => { return Err(ParseError { message: format!("Error parse json '{}'", e.to_string()) }); }
        };
        Ok(Self::from_json(&json_object).await?)
    }
}


#[rocket::async_trait]
impl<'r> FromData<'r> for DictionaryType {
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

        match DictionaryType::from_str(string.as_str()).await {
            Ok(ot) => {
                if !getToken(req, &ot).authorize(&user) {
                    return Failure((Status { code: 403 }, Self::Error { message: "Error".to_string() }));
                }
                Success(ot)
            }
            Err(e) => { Failure((Status { code: 500 }, Self::Error { message: "Error".to_string() })) }
        }
    }
}