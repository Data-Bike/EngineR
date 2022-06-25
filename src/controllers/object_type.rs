use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::object::repository::repository::Repository;




#[post("/add", data = "<object_type>")]
async fn add_object_type(object_type: ObjectType) -> RawJson<String> {
    let res = Repository::createObjectType(object_type).await.ok();
    match res {
        None => { RawJson(format!("ERROR")) }
        Some(_) => {
            RawJson("OK".to_string())
        }
    }
}


#[get("/get/<id>")]
async fn get_object_type(id: usize) -> RawHtml<String> {
    println!("Start getting object by id");
    let object_type = Repository::getObjectTypeFromId(id.to_string()).await.ok();
    println!("Got object");
    match object_type {
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
        rocket.mount("/object_type", routes![hello,add_object_type,get_object_type])
    })
}

// #[cfg(test)]
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

    fn add_object_type() {
        match block_on(Object_Repository::getObjectTypeFromAlias("fl".to_string())) {
            Ok(_) => { return; }
            Err(_) => {}
        };

        match block_on(Object_Repository::createObjectType(ObjectType {
            id: None,
            fields: vec![
                Field {
                    id: None,
                    alias: "lastname".to_string(),
                    kind: "varchar(255)".to_string(),
                    name: "lastname".to_string(),
                    default: None,
                    value: None,
                    require: false,
                    index: false,
                    preview: false,
                },
                Field {
                    id: None,
                    alias: "firstname".to_string(),
                    kind: "varchar(255)".to_string(),
                    name: "firstname".to_string(),
                    default: None,
                    value: None,
                    require: false,
                    index: false,
                    preview: false,
                },
                Field {
                    id: None,
                    alias: "patronymic".to_string(),
                    kind: "varchar(255)".to_string(),
                    name: "patronymic".to_string(),
                    default: None,
                    value: None,
                    require: false,
                    index: false,
                    preview: false,
                },
                Field {
                    id: None,
                    alias: "birthday".to_string(),
                    kind: "timestamp".to_string(),
                    name: "birthday".to_string(),
                    default: None,
                    value: None,
                    require: false,
                    index: false,
                    preview: false,
                },
            ],
            kind: "object".to_string(),
            alias: "fl".to_string(),
        })) {
            Ok(_) => {}
            Err(e) => {
                println!("{:?}", e);
                panic!("Error create object type");
            }
        };
    }


    fn get_user_groups() -> Vec<Group> {
        let g = Group {
            alias: "acc_object".to_string(),
            name: "acc_object".to_string(),
            level: "system".to_string(),
            id: Some("1".to_string()),
            permissions: PermissionsGroup {
                system: vec![Permission {
                    access: Access::allow,
                    alias: "system_of_object_access".to_string(),
                    id: None,
                    level: PermissionLevel::system,
                    kind: PermissionKind::create,
                    name: "system_of_object_access".to_string(),
                    object: "object".to_string(),
                }],
                object: vec![],
                object_type: vec![Permission {
                    access: Access::allow,
                    alias: "Access to FL".to_string(),
                    id: None,
                    level: PermissionLevel::object_type,
                    kind: PermissionKind::create,
                    name: "system_of_object_access".to_string(),
                    object: "1".to_string(),
                }],
                object_type_field: vec![
                    Permission {
                        access: Access::allow,
                        alias: "Access to Lastname".to_string(),
                        id: None,
                        level: PermissionLevel::object_type_field,
                        kind: PermissionKind::create,
                        name: "Access to Lastname".to_string(),
                        object: "1".to_string(),
                    },
                    Permission {
                        access: Access::allow,
                        alias: "Access to Firstname".to_string(),
                        id: None,
                        level: PermissionLevel::object_type_field,
                        kind: PermissionKind::create,
                        name: "Access to Firstname".to_string(),
                        object: "2".to_string(),
                    },
                    Permission {
                        access: Access::allow,
                        alias: "Access to Patronymic".to_string(),
                        id: None,
                        level: PermissionLevel::object_type_field,
                        kind: PermissionKind::create,
                        name: "Access to Patronymic".to_string(),
                        object: "3".to_string(),
                    },
                    Permission {
                        access: Access::allow,
                        alias: "Access to birthday".to_string(),
                        id: None,
                        level: PermissionLevel::object_type_field,
                        kind: PermissionKind::create,
                        name: "Access to birthday".to_string(),
                        object: "4".to_string(),
                    },
                ],
                link: vec![],
                link_type: vec![],
            },
        };
        match block_on(Secure_Repository::createGroup(&g)) {
            Ok(_) => {}
            Err(e) => {
                println!("{:?}", e);
                panic!("Error create group");
            }
        };
        vec![g]
    }

    pub fn add_test_user() {
        match block_on(User_Repository::getUserByLogin("root".to_string())) {
            Ok(_) => { return; }
            Err(_) => {}
        };

        let user = User {
            id: None,
            login: "root".to_string(),
            password: match bcrypt::hash("testestest".to_string(), DEFAULT_COST) {
                Ok(h) => { h }
                Err(e) => { panic!("Cannt hashed password"); }
            },
            access_token: "".to_string(),
            oauth: "".to_string(),
            groups: get_user_groups(),
            date_last_active: None,
            date_registred: Utc::now().naive_utc(),
        };
        let res = block_on(User_Repository::createUser(&user));
        assert_eq!(res, Ok("1".to_string()));
    }

    fn login() -> String {
        add_test_user();
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let mut response = client.post(uri!("/login")).body("{\
            \"login\":\"root\",\
            \"password\":\"testestest\"\
        }").remote(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
            .dispatch();
        let cookie_str = response
            .headers()
            .get("Set-Cookie")
            .collect::<Vec<&str>>()
            .pop()
            .unwrap()
            .to_string();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.headers().get("Set-Cookie").count() >= 1);
        assert_eq!(response.into_string(), Some("OK".to_string()));
        return cookie_str;
    }


    #[test]
    fn add_object_type_test() {
        let session_cookie = login();
        println!("Set cookie: '{}'", session_cookie.as_str());
        let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.post(uri!("/object/add_object_type")).body("{\
                \"fields\":[
                    {
                        \"id\":\"1\",
                        \"alias\":\"code\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"code\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    },
                    {
                        \"id\":\"2\",
                        \"alias\":\"number\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"number\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }
                ],
                \"kind\":\"object\",
                \"alias\":\"tl\"
            }");
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", "1").secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn get_object_type_test() {
        let session_cookie = login();
        println!("Set cookie: '{}'", session_cookie.as_str());
        let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/object/get_object_type/1"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", "1").secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "{\"alias\":\"fl\",\"fields\":[{\"alias\":\"lastname\",\"default\":\"varchar(255)\",\"id\":\"1\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"lastname\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"birthday\",\"default\":\"timestamp\",\"id\":\"2\",\"index\":false,\"kind\":\"timestamp\",\"name\":\"birthday\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"firstname\",\"default\":\"varchar(255)\",\"id\":\"3\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"firstname\",\"preview\":false,\"require\":false,\"value\":null},{\"alias\":\"patronymic\",\"default\":\"varchar(255)\",\"id\":\"4\",\"index\":false,\"kind\":\"varchar(255)\",\"name\":\"patronymic\",\"preview\":false,\"require\":false,\"value\":null}],\"id\":\"1\",\"kind\":\"object\"}");
    }
}