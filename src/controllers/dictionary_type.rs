use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::dictionary::entity::dictionary::DictionaryType;
use crate::model::dictionary::repository::repository::Repository;
use crate::model::error::RepositoryError;
use crate::model::object::entity::object::{Object, ObjectType};


#[get("/get/<id>")]
async fn get_dictionary_type(id: usize) -> RawHtml<String> {
    println!("Start getting object by id");
    let object = match Repository::getDictionaryTypeById(id.to_string()).await {
        Ok(dt) => { Some(dt) }
        Err(e) => { return RawHtml(format!("ERROR {:?}", e)); }
    };
    println!("Got object");
    match object {
        None => { RawHtml(format!("ERROR")) }
        Some(dt) => {
            println!("Object to json");
            RawHtml(
                match to_value(dt) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml("ERROR".to_string()); }
                }.to_string()
            )
        }
    }
}

#[post("/add", data = "<dictionary_type>")]
async fn add_dictionary_type(dictionary_type: DictionaryType) -> RawJson<String> {
    println!("add_dictionary_type {:?}", dictionary_type);
    let id = Repository::createDictionaryType(&dictionary_type).await.ok();
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
        rocket.mount("/dictionary_type", routes![get_dictionary_type,add_dictionary_type,hello])
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
        let mut response = client.get(uri!("/dictionary_type/hello")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn add_dictionary_type() {
        add_object_type();
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut request = client.post(uri!("/dictionary_type/add")).body("{\
            \"dictionaries\":[
                {
                    \"alias\":\"yes\",
                    \"name\":\"yes\"
                },
                {
                    \"alias\":\"no\",
                    \"name\":\"no\"
                }
            ],
            \"name\":\"yesno\",
            \"alias\":\"yesno\"
        }");
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn get_dictionary_type() {
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/dictionary_type/get/1"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "{\"alias\":\"yesno\",\"dictionaries\":[{\"alias\":\"yes\",\"id\":\"1\",\"name\":\"yes\"},{\"alias\":\"no\",\"id\":\"2\",\"name\":\"no\"}],\"id\":\"1\",\"name\":\"yesno\"}");
    }
}