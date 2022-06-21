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
        // .domain("")
        // .path("/")
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
            None => { return RawJson("Failed cookie".to_string()); }
            Some(c) => {
                jar.add_private(c);
                return RawJson("OK".to_string());
            }
        }
    }

    RawJson("Failed access".to_string())
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


#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use bcrypt::DEFAULT_COST;
    use chrono::Utc;
    use futures::executor::block_on;
    use rocket::local::blocking::Client;
    use rocket::http::Status;
    use rocket::uri;
    use crate::{rocket_build};
    use crate::controllers::form_parser::error::ParseError;
    use crate::model::user::entity::user::User;
    use crate::model::user::repository::repository::Repository as User_Repository;


    pub fn add_test_user() {

        let user = User {
            id: None,
            login: "root".to_string(),
            password: match bcrypt::hash("testestest".to_string(), DEFAULT_COST) {
                Ok(h) => { h }
                Err(e) => { panic!("Cannt hashed password"); }
            },
            access_token: "".to_string(),
            oauth: "".to_string(),
            groups: vec![],
            date_last_active: None,
            date_registred: Utc::now().naive_utc(),
        };
        let res = block_on(User_Repository::createUser(&user));
        assert_eq!(res, Ok("1".to_string()));
    }

    #[test]
    fn login() {
        add_test_user();
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.post(uri!("/login")).body("{\
            \"login\":\"root\",\
            \"password\":\"testestest\"\
        }").remote(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.headers().get("Set-Cookie").count() >= 1);
        assert_eq!(response.into_string(), Some("OK".to_string()));
    }
}