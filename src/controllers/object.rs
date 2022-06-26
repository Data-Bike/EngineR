use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::object::repository::repository::Repository;


#[get("/get/<id>")]
async fn get_object(id: usize) -> RawHtml<String> {
    println!("Start getting object by id");
    let object = Repository::hydrateFilledObjectType(id.to_string()).await.ok();
    println!("Got object");
    match object {
        None => { RawHtml(format!("ERROR")) }
        Some(o) => {
            println!("Object to json");
            RawHtml(
                match to_value(o) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml("ERROR".to_string()); }
                }.to_string()
            )
        }
    }
}

#[post("/add", data = "<object>")]
async fn add_object(object: Object) -> RawJson<String> {
    let id = Repository::createObject(&object).await.ok();
    match id {
        None => { RawJson(format!("ERROR")) }
        Some(i) => { RawJson(i) }
    }
}

#[post("/search", data = "<object>")]
async fn search_object(object: Object) -> RawJson<String> {
    let res = Repository::searchObject(&object).await.ok();
    match res {
        None => { RawJson(format!("ERROR")) }
        Some(r) => {
            RawJson(
                match to_value(r) {
                    Ok(x) => { x }
                    Err(e) => { return RawJson("ERROR".to_string()); }
                }.to_string())
        }
    }
}


#[get("/hello")]
async fn hello() -> RawJson<String> {
    return RawJson(format!("Hello!"));
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/object", routes![get_object,add_object,hello])
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
        let mut response = client.get(uri!("/object/hello")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn add_object() {
        add_object_type();
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut request = client.post(uri!("/object/add")).body("{\
            \"filled\":{\
                \"id\":\"1\",
                \"fields\":[
                    {
                        \"id\":\"1\",
                        \"alias\":\"lastname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"lastname\",
                        \"value\":\"Platonov\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"2\",
                        \"alias\":\"firstname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"firstname\",
                        \"value\":\"Alexander\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"3\",
                        \"alias\":\"patronymic\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"patronymic\",
                        \"value\":\"Alexanderovich\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"4\",
                        \"alias\":\"birthday\",
                        \"kind\":\"timestamp\",
                        \"name\":\"datetime\",
                        \"value\":\"1988-03-02T02:00:00.00Z\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }
                ],
                \"kind\":\"object\",
                \"alias\":\"fl\"
            },\
            \"date_created\":\"1988-03-02T02:30:00.00Z\",\
            \"user_created\":\"1\",
            \"hash\":\"\"\
        }");
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn get_object() {
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/object/get/1"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }
}