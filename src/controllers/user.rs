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
                },Permission {
                    access: Access::allow,
                    alias: "system_of_object_access".to_string(),
                    id: None,
                    level: PermissionLevel::system,
                    kind: PermissionKind::create,
                    name: "system_of_object_access".to_string(),
                    object: "user".to_string(),
                },Permission {
                    access: Access::allow,
                    alias: "system_of_object_access".to_string(),
                    id: None,
                    level: PermissionLevel::system,
                    kind: PermissionKind::create,
                    name: "system_of_object_access".to_string(),
                    object: "object_type".to_string(),
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
    fn add_user() {
        add_object_type();
        let session_cookie = login();
        println!("Set cookie: '{}'", session_cookie.as_str());
        let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.post(uri!("/user/add")).body("{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"1996-12-19T16:39:57-08:00\",\"groups\":[\"1\"],\"login\":\"root2\",\"oauth\":\"\",\"password\":\"$2b$12$4H2xurOmbAlkrk2ZcgbeBe4RgaPk23D118IhYLv1kLOCBCPIDxj62\"}");
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", "1").secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }

    #[test]
    fn get_user() {
        let session_cookie = login();
        println!("Set cookie: '{}'", session_cookie.as_str());
        let h = Header::new("Cookie", session_cookie);
        let client = Client::tracked(rocket_build()).expect("valid rocket instance");
        let request = client.get(uri!("/user/get/2"));
        // request.add_header(h);
        let cookie = CookieBuilder::new("user_id", "1").secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "{\"access_token\":\"\",\"date_last_active\":null,\"date_registred\":\"2022-06-21T17:13:18.039234\",\"groups\":[{\"alias\":\"acc_object\",\"id\":\"1\",\"level\":\"system\",\"name\":\"acc_object\",\"permissions\":{\"link\":[],\"link_type\":[],\"object\":[],\"object_type\":[{\"access\":\"allow\",\"alias\":\"Access to FL\",\"id\":\"4\",\"kind\":\"create\",\"level\":\"object_type\",\"name\":\"system_of_object_access\",\"object\":\"1\"}],\"object_type_field\":[{\"access\":\"allow\",\"alias\":\"Access to Firstname\",\"id\":\"1\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Firstname\",\"object\":\"2\"},{\"access\":\"allow\",\"alias\":\"Access to Lastname\",\"id\":\"6\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Lastname\",\"object\":\"1\"},{\"access\":\"allow\",\"alias\":\"Access to birthday\",\"id\":\"5\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to birthday\",\"object\":\"4\"},{\"access\":\"allow\",\"alias\":\"Access to Patronymic\",\"id\":\"2\",\"kind\":\"create\",\"level\":\"object_type_field\",\"name\":\"Access to Patronymic\",\"object\":\"3\"}],\"system\":[{\"access\":\"allow\",\"alias\":\"system_of_object_access\",\"id\":\"3\",\"kind\":\"create\",\"level\":\"system\",\"name\":\"system_of_object_access\",\"object\":\"object\"}]}}],\"id\":\"1\",\"login\":\"root\",\"oauth\":\"\",\"password\":\"$2b$12$4H2xurOmbAlkrk2ZcgbeBe4RgaPk23D118IhYLv1kLOCBCPIDxj62\"}");
    }
}