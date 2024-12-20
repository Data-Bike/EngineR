use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository::Repository;


#[get("/get/<id>")]
async fn get_user(id: usize) -> RawHtml<String> {
    println!("Start getting user by id");
    let object = Repository::getUserById(id.to_string()).await.ok();
    println!("Got user");
    match object {
        None => { RawHtml(format!("ERROR cannt get user")) }
        Some(o) => {
            println!("User to json");
            RawHtml(
                match to_value(o) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml(format!("ERROR serialize user {}",e)); }
                }.to_string()
            )
        }
    }
}

#[post("/add", data = "<user>")]
async fn add_user(user: User) -> RawJson<String> {
    let id = Repository::createUser(&user).await.ok();
    match id {
        None => { RawJson(format!("ERROR")) }
        Some(i) => { RawJson(i) }
    }
}


#[get("/hello")]
async fn hello() -> RawJson<String> {
    return RawJson(format!("Hello!"));
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/user", routes![get_user,add_user,hello])
    })
}

// #[cfg(object)]
// mod tests;

#[cfg(test)]
mod test {
    use std::future::Future;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use async_std::task::block_on;
    use bcrypt::DEFAULT_COST;
    use chrono::Utc;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use rocket::local::blocking::Client;
    use rocket::http::{Header, Status};
    use rocket::http::private::cookie::CookieBuilder;
    use rocket::log::private::kv::Source;
    use rocket::uri;
    use crate::{rocket_build};
    use crate::controllers::test::{add_object_type, login};
    use crate::model::error::RepositoryError;
    use crate::model::object::entity::object::{Field, ObjectType};
    use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
    use crate::model::user::entity::user::User;
    use crate::model::user::repository::repository::Repository as User_Repository;
    use crate::model::object::repository::repository::Repository as Object_Repository;
    use crate::model::secure::repository::repository::Repository as Secure_Repository;



    #[test]
    fn hello() {
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.get(uri!("/user/hello")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn add_user() {
        add_object_type();
        let user = login();
        let login = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect::<String>();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");

        let request = client.post(uri!("/user/add")).body(format!("{{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"1996-12-19T16:39:57.123456\",\"groups\":[\"1\"],\"login\":\"root2_{}\",\"oauth\":\"\",\"password\":\"$2b$12$4H2xurOmbAlkrk2ZcgbeBe4RgaPk23D118IhYLv1kLOCBCPIDxj62\"}}",login));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn get_user() {
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/user/get/1"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response_str = response.into_string().unwrap();
        println!("response_str: {}",response_str);
        let u_gotten =
            block_on(
                User::from_str(response_str.as_str())
            ).unwrap();
        let u_saved = block_on(User_Repository::getUserById("1".to_string())).unwrap();
        assert_eq!(u_gotten.id, u_saved.id);
        // assert_eq!(response.into_string().unwrap(), "{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"2022-06-26T13:12:27.323376\",\"groups\":[],\"id\":\"1\",\"login\":\"root\",\"oauth\":\"\",\"password\":\"$2b$12$Cv/IRehHUxGhobc5KiZmNumQmauD3qe6EaQ6lNfw2LLxJmrENCy0G\"}");
    }
}