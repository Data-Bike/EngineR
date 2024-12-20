use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::link::repository::repository::Repository;


#[post("/add", data = "<link>")]
async fn add_link(link: Link) -> RawJson<String> {
    let res = Repository::setLink(link).await.ok();
    match res {
        None => { RawJson(format!("ERROR")) }
        Some(_) => {
            RawJson("OK".to_string())
        }
    }
}


#[get("/get/<id>")]
async fn get_link(id: usize) -> RawHtml<String> {
    println!("Start getting object by id");
    let link = Repository::getLinkById(id.to_string()).await.ok();
    println!("Got object");
    match link {
        None => { RawHtml(format!("ERROR")) }
        Some(ot) => {
            println!("Object to json");
            RawHtml(
                match to_value(ot) {
                    Ok(x) => { x }
                    Err(e) => { return RawHtml("ERROR".to_string()); }
                }.to_string()
            )
        }
    }
}

#[get("/hello")]
async fn hello() -> RawJson<String> {
    return RawJson(format!("Hello!"));
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managing objects", move |rocket| async move {
        rocket.mount("/link", routes![hello,add_link,get_link])
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
    use crate::controllers::test::{add_link_type, login};
    use crate::model::error::RepositoryError;
    use crate::model::link::entity::link::Link;
    use crate::model::link::repository::repository::Repository;
    use crate::model::object::entity::object::{Field, ObjectType};
    use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
    use crate::model::user::entity::user::User;
    use crate::model::user::repository::repository::Repository as User_Repository;
    use crate::model::object::repository::repository::Repository as Object_Repository;
    use crate::model::secure::repository::repository::Repository as Secure_Repository;


    #[test]
    fn hello() {
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.get(uri!("/link/hello")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Hello!");
    }


    #[test]
    fn add_link_test() {
        let user = login();
        let lt = add_link_type();

        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.post(uri!("/link/add")).body(format!("{{\
                \"object_from_id\":\"1\",
                \"object_to_id\":\"2\",
                \"user_created_id\":\"1\",
                \"link_type_id\":\"{}\",
                \"date_created\":\"2022-03-02T02:00:00.00Z\",
                \"object_from_id\":\"1\"
            }}", lt.id.unwrap()));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn get_link_test() {
        let user = login();
        // println!("Set cookie: '{}'", session_cookie.as_str());
        // let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/link/get/1"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", user.id.unwrap()).secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);

        // let l_gotten =
        //     block_on(
        //         Link::from_str(response.into_string().unwrap().as_str())
        //     ).unwrap();
        // let l_saved = block_on(Repository::getLinkById("1".to_string())).unwrap();
        // assert_eq!(l_gotten, l_saved);
        // assert_eq!(response.into_string().unwrap(), "{\"alias\":\"fl\",\"fields\":[{\"alias\":\"lastname\",\"default\":\"varchar(255)\",\"id\":\"1\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"lastname\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"birthday\",\"default\":\"timestamp\",\"id\":\"2\",\"index\":false,\"kind\":\"timestamp\",\"name\":\"birthday\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"firstname\",\"default\":\"varchar(255)\",\"id\":\"3\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"firstname\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"patronymic\",\"default\":\"varchar(255)\",\"id\":\"4\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"patronymic\",\"preview\":false,\"require\":false,\"value\":null}],\"id\":\"1\",\"kind\":\"object\"}");
    }
}