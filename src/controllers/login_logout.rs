use rocket::{routes, time};
use rocket::response::content::{RawJson};
use rocket::post;
use rocket::fairing::AdHoc;
use crate::model::user::repository::repository::Repository as User_repository;
use crate::controllers::secure::authentication::token::Token;
use rocket::http::{Cookie, CookieJar};
use crate::controllers::form_parser::error::ParseError;


async fn build_cookie(token: &Token) -> Result<Cookie<'static>, ParseError> {
    let user = User_repository::getUserByLogin(token.credentials.login.clone()).await?;
    Ok(Cookie::<'static>::build("user_id", match user.id {
        None => { return Err(ParseError { message: "User request must has id".to_string() }); }
        Some(c) => { c }
    })
        .domain("")
        .path("/")
        .secure(true)
        .max_age(time::Duration::days(1))
        .http_only(true)
        .finish())
}

#[post("/login", data = "<token>", )]
async fn login(token: Token, jar: &CookieJar<'_>) -> RawJson<String> {
    if token.is_allow() {
        let cookie = build_cookie(&token).await.ok();
        match cookie {
            None => { return RawJson("Failed".to_string()); }
            Some(c) => {
                jar.add_private(c);
                return RawJson("OK".to_string());
            }
        }
    }

    RawJson("Failed".to_string())
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
