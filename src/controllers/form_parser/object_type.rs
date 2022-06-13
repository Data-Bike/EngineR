use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use async_std::task::block_on;
use sqlx::Error as Sqlx_Error;
use chrono::{DateTime, Utc};
use rocket::data::{FromData, ToByteUnit};
use rocket::{Data, Request, request};
use rocket::error::ErrorKind::Io;
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{FromRequest, Outcome};
use serde::de::{Expected, Unexpected};
use serde_json::{from_str, Value};
use crate::controllers::secure::authorization::token::{EmptyToken, Token};
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

pub fn getToken(req: &Request<'_>, object_type: &ObjectType) -> Token {
    let requestKind = match req.method() {
        Method::Get => { PermissionKind::read }
        Method::Post => { PermissionKind::edit }
        _ => { PermissionKind::read }
    };

    let system = req.uri().path().segments().get(0).unwrap().to_string();
    Token::fromObjectType(requestKind, system, object_type)
}

impl Field {
    pub fn from_json(json_object: &Value) -> Result<Self, ParseError> {
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
        macro_rules! err_resolve_bool {
            ( $x:expr, $key:expr ) => {
                match match $x.get($key) {
                    None => { return Err(ParseError { message: format!("Error {} not found",$key) }); }
                    Some(v) => { v }
                }.as_bool() {
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

        let alias = err_resolve!(json_object,"alias").to_string();
        let kind = err_resolve!(json_object,"kind").to_string();
        let name = err_resolve!(json_object,"name").to_string();
        let require = err_resolve_bool!(json_object,"require");
        let index = err_resolve_bool!(json_object,"index");
        let preview = err_resolve_bool!(json_object,"preview");
        let default = err_resolve_option!(json_object,"default");
        let value = err_resolve_option!(json_object,"value");


        Ok(Field {
            id,
            alias,
            kind,
            name,
            default,
            value,
            require,
            index,
            preview,
        })
    }
}

impl ObjectType {
    pub fn from_json(json_object: &Value) -> Result<Self, ParseError> {
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

        let fields = match json_object.get("fields") {
            None => { return Err(ParseError { message: format!("Error {} not found", "fields") }); }
            Some(v) => {
                match v.as_array() {
                    None => { return Err(ParseError { message: "Error fields is not array".to_string() }); }
                    Some(v) => {
                        v
                            .iter()
                            .map(|f| Field::from_json(f).unwrap())
                            .collect()
                    }
                }
            }
        };
        let kind = err_resolve!(json_object,"kind").to_string();
        let alias = err_resolve!(json_object,"alias").to_string();

        Ok(ObjectType {
            id,
            fields,
            kind,
            alias,
        })
    }
    pub async fn from_str(string: &str) -> Result<Self, ParseError> {
        let json_object: Value = match from_str::<Value>(string) {
            Ok(v) => { v }
            Err(e) => { return Err(ParseError { message: format!("Error parse json '{}'", e.to_string()) }); }
        };
        Self::from_json(&json_object)
    }
}


#[rocket::async_trait]
impl<'r> FromData<'r> for ObjectType {
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

        match ObjectType::from_str(string.as_str()).await {
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