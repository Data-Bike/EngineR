use std::convert::Infallible;
use std::num::ParseIntError;
use rocket::{Request};

use async_std::task::block_on;
use rocket::http::Status;


use serde_json::{from_str, Value};


use crate::model::secure::entity::permission::Group;
use crate::model::user::repository::repository::Repository as User_repository;
use crate::model::secure::repository::repository::Repository as Secure_repository;
use crate::model::user::entity::user::User;

use rocket::outcome::IntoOutcome;
use rocket::outcome::Outcome::{Failure, Success};
use rocket::request::{self, FromRequest};
use crate::controllers::form_parser::error::ParseError;

struct Range {
    from: Option<u64>,
    to: Option<u64>,
}


#[rocket::async_trait]
impl<'r> FromRequest<'r> for Range {
    type Error = ParseError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Range, ParseError> {
        match request.headers().get_one("range") {
            None => { Success(Range { from: None, to: None }) }
            Some(r) => {
                let s = r.split('=');
                if s.clone().count() != 2 {
                    return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range".to_string() }));
                }

                let rs = s.collect::<Vec<&str>>();
                let units = match rs.get(0) {
                    None => { return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range".to_string() })); }
                    Some(u) => { *u }
                };

                if units != "items" {
                    return Failure((Status { code: 500 }, ParseError { message: "Units` header range must be 'items'".to_string() }));
                };

                let range_str = match rs.get(1) {
                    None => { return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range".to_string() })); }
                    Some(u) => { *u }
                };

                let test_split_range_str = range_str.split(',');
                if test_split_range_str.count() != 1 {
                    return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range".to_string() }));
                }
                let split_range_str = range_str.split('-').collect::<Vec<&str>>();

                let from = match split_range_str.get(0).and_then(|f| Some(f.parse::<u64>())).transpose() {
                    Ok(x) => { x }
                    Err(_) => { return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range 'from must be integer'".to_string() })); }
                };

                let to = match split_range_str.get(1).and_then(|f| Some(f.parse::<u64>())).transpose() {
                    Ok(x) => { x }
                    Err(_) => { return Failure((Status { code: 500 }, ParseError { message: "Error cannot parse header range 'to must be integer'".to_string() })); }
                };


                Success(Range { from, to })
            }
        }
    }
}
