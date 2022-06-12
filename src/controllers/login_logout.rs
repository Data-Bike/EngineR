use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, SystemTimeError};
use chrono::{Utc};

use rocket::{routes, State, time};

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
use crate::model::user::repository::repository::Repository as User_repository;
use crate::controllers::form_parser::object;
use crate::controllers::secure::authentication::token::Token;
use rocket::http::{Cookie, CookieJar};
use time::{Duration, OffsetDateTime};


async fn build_cookie(token: &Token) -> Cookie<'static> {
    let user = User_repository::getUserByLogin(token.credentials.login.clone()).await;
    Cookie::<'static>::build("user_id", user.id.unwrap())
        .domain("")
        .path("/")
        .secure(true)
        .max_age(time::Duration::days(1))
        .http_only(true)
        .finish()
}

#[post("/login", data = "<token>", )]
async fn login(token: Token, jar: &CookieJar<'_>) -> RawJson<String> {
    if token.is_allow() {
        let cookie = build_cookie(&token).await;
        jar.add_private(cookie);
    }

    RawJson("OK".to_string())
}

#[post("/logout")]
async fn logout(jar: &CookieJar<'_>) -> RawJson<String> {
    match jar.get_private("user_id") {
        None => { return RawJson("OK".to_string()); }
        Some(c) => { jar.remove_private(c) }
    };
    RawJson("OK".to_string())
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing login\\logout", move |rocket| async move {
        rocket.mount("/", routes![login,logout])
    })
}
