use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{routes, State};

use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::launch;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
// use crate::init::model::{Database, ModelApp, Secure, User};
use crate::model::user::entity::user::User;
// use rocket_sync_db_pools::database;
// use rocket_sync_db_pools::postgres;
use crate::model;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository;
use crate::controllers::form_parser::object;

