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
        None => { RawHtml(format!("ERROR")) }
        Some(o) => {
            println!("User to json");
            RawHtml(
                match to_value(o) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml("ERROR".to_string()); }
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
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.post(uri!("/user/add")).body("{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"1996-12-19T16:39:57-08:00\",\"groups\":[\"1\"],\"login\":\"root2\",\"oauth\":\"\",\"password\":\"$2b$12$4H2xurOmbAlkrk2ZcgbeBe4RgaPk23D118IhYLv1kLOCBCPIDxj62\"}");
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
        let request = client.get(uri!("/user/get/2"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"2022-06-21T17:13:18.039234\",\"groups\":[{\"alias\":\"acc_object\",\"id\":\"1\",\"level\":\"system\",\"name\":\"acc_object\",\"permissions\":{\"link\":[],\"link_type\":[],\"object\":[],\"object_type\":[{\"access\":\"allow\",\"alias\":\"Access to FL\",\"id\":\"4\",\"kind\":\"create\",\"level\":\"object_type\",\"name\":\"system_of_object_access\",\"object\":\"1\"}],\"object_type_field\":[{\"access\":\"allow\",\"alias\":\"Access to Firstname\",\"id\":\"1\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Firstname\",\"object\":\"2\"},{\"access\":\"allow\",\"alias\":\"Access to Lastname\",\"id\":\"6\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Lastname\",\"object\":\"1\"},{\"access\":\"allow\",\"alias\":\"Access to birthday\",\"id\":\"5\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to birthday\",\"object\":\"4\"},{\"access\":\"allow\",\"alias\":\"Access to Patronymic\",\"id\":\"2\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Patronymic\",\"object\":\"3\"}],\"system\":[{\"access\":\"allow\",\"alias\":\"system_of_object_access\",\"id\":\"3\",\"kind\":\"create\",\"level\":\"system\",\"name\":\"system_of_object_access\",\"object\":\"object\"}]}}],\"id\":\"1\",\"login\":\"root\",\"oauth\":\"\",\"password\":\"$2b$12$4H2xurOmbAlkrk2ZcgbeBe4RgaPk23D118IhYLv1kLOCBCPIDxj62\"}");
    }
}