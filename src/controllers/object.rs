use rocket::{routes};
use rocket::response::content::{RawHtml, RawJson};
use rocket::get;
use rocket::post;
use rocket::fairing::AdHoc;
use serde_json::value::to_value;
use crate::model::object::entity::object::Object;
use crate::model::object::repository::repository::Repository;


#[get("/get/<id>")]
async fn get_object(id: usize) -> RawHtml<String> {
    let object = Repository::hydrateFilledObjectType(id.to_string()).await.ok();
    match object {
        None => { RawHtml(format!("ERROR")) }
        Some(o) => {
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
                    kind: "datetime".to_string(),
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
    fn add_object() {
        add_object_type();
        let session_cookie = login();
        println!("Set cookie: '{}'", session_cookie.as_str());
        let h = Header::new("Cookie", session_cookie);
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
                        \"kind\":\"varchar(255)\",
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
        let cookie = CookieBuilder::new("user_id", "1").secure(true);
        let response = request.private_cookie(cookie.finish()).dispatch();


        assert_eq!(response.status(), Status::Ok);
        // assert_eq!(response.into_string().unwrap(), "Hello!");
    }
}