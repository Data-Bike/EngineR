use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use async_std::task::block_on;
use bcrypt::DEFAULT_COST;
use chrono::{NaiveDateTime, Utc};
use rocket::local::blocking::Client;
use rocket::http::{Header, Status};
use rocket::http::private::cookie::CookieBuilder;
use rocket::log::private::kv::Source;
use rocket::uri;
use crate::{rocket_build};
use crate::model::error::RepositoryError;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::{Access, Group, Permission, PermissionKind, PermissionLevel, PermissionsGroup};
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository::Repository as User_Repository;
use crate::model::link::repository::repository::Repository as Link_Repository;
use crate::model::object::repository::repository::Repository as Object_Repository;
use crate::model::secure::repository::repository::Repository as Secure_Repository;
use rand::{distributions::Alphanumeric, Rng};
use rocket::figment::map;
use rocket::figment::value::Map;
use rocket::http::private::cookie::Expiration::DateTime;
use serde_json::{to_value, Value};
use crate::model::link::entity::link::LinkType;

pub fn add_link_type() -> LinkType {
    let id_from = add_object_type_rand_alias();
    let id_to = add_object_type_rand_alias();
    let alias = format!("test_link_type_{}", rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>());
    let name = format!("Test link type {}", alias);
    let object_type_from = block_on(Object_Repository::getObjectTypeFromId(id_from)).unwrap();
    let object_type_to = block_on(Object_Repository::getObjectTypeFromId(id_to)).unwrap();

    let lt = LinkType {
        id: None,
        alias,
        name,
        object_type_from,
        object_type_to,
    };

    let res = block_on(Link_Repository::createLinkType(&lt)).unwrap();
    return res;
}


pub fn add_object_type() {
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
                dictionary_type: None,
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
                dictionary_type: None,
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
                dictionary_type: None,
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
                dictionary_type: None,
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

pub fn add_object_type_rand_alias() -> String {
    let mut alias = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>();

    while match block_on(Object_Repository::getObjectTypeFromAlias(alias.to_string())) {
        Ok(_) => { true }
        Err(_) => { false }
    } {
        alias = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect::<String>();
    }

    let id = match block_on(Object_Repository::createObjectType(ObjectType {
        id: None,
        fields: vec![
            Field {
                id: None,
                alias: format!("lastname_{}", alias),
                kind: "varchar(255)".to_string(),
                name: format!("lastname_{}", alias),
                default: None,
                value: None,
                dictionary_type: None,
                require: false,
                index: false,
                preview: false,
            },
            Field {
                id: None,
                alias: format!("firstname_{}", alias),
                kind: "varchar(255)".to_string(),
                name: format!("firstname_{}", alias),
                default: None,
                value: None,
                dictionary_type: None,
                require: false,
                index: false,
                preview: false,
            },
            Field {
                id: None,
                alias: format!("patronymic_{}", alias),
                kind: "varchar(255)".to_string(),
                name: format!("patronymic_{}", alias),
                default: None,
                value: None,
                dictionary_type: None,
                require: false,
                index: false,
                preview: false,
            },
            Field {
                id: None,
                alias: format!("birthday_{}", alias),
                kind: "timestamp".to_string(),
                name: format!("birthday_{}", alias),
                default: None,
                value: None,
                dictionary_type: None,
                require: false,
                index: false,
                preview: false,
            },
        ],
        kind: "object".to_string(),
        alias: format!("fl_{}", alias),
    })) {
        Ok(i) => { i }
        Err(e) => {
            println!("{:?}", e);
            panic!("Error create object type");
        }
    };
    id
}


pub fn get_user_groups() -> Vec<Group> {
    let mut g = Group {
        alias: format!("group_of_{}_access_", rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect::<String>()),
        name: "acc_object".to_string(),
        level: "system".to_string(),
        id: Some("1".to_string()),
        permissions: PermissionsGroup {
            system: vec![Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::system,
                kind: PermissionKind::create,
                name: "system_of_object_access".to_string(),
                object: "object".to_string(),
            }, Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::system,
                kind: PermissionKind::create,
                name: "system_of_object_access".to_string(),
                object: "user".to_string(),
            }, Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::system,
                kind: PermissionKind::create,
                name: "system_of_object_access".to_string(),
                object: "object_type".to_string(),
            }, Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::system,
                kind: PermissionKind::create,
                name: "system_of_object_access".to_string(),
                object: "link".to_string(),
            }, Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::system,
                kind: PermissionKind::create,
                name: "system_of_object_access".to_string(),
                object: "dictionary_type".to_string(),
            }],
            object: vec![],
            object_type: vec![Permission {
                access: Access::allow,
                alias: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                id: None,
                level: PermissionLevel::object_type,
                kind: PermissionKind::create,
                name: format!("system_of_{}_access_", rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect::<String>()),
                object: "1".to_string(),
            }],
            object_type_field: vec![
                Permission {
                    access: Access::allow,
                    alias: format!("system_of_{}_access_", rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect::<String>()),
                    id: None,
                    level: PermissionLevel::object_type_field,
                    kind: PermissionKind::create,
                    name: "Access to Lastname".to_string(),
                    object: "1".to_string(),
                },
                Permission {
                    access: Access::allow,
                    alias: format!("system_of_{}_access_", rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect::<String>()),
                    id: None,
                    level: PermissionLevel::object_type_field,
                    kind: PermissionKind::create,
                    name: "Access to Firstname".to_string(),
                    object: "2".to_string(),
                },
                Permission {
                    access: Access::allow,
                    alias: format!("system_of_{}_access_", rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect::<String>()),
                    id: None,
                    level: PermissionLevel::object_type_field,
                    kind: PermissionKind::create,
                    name: "Access to Patronymic".to_string(),
                    object: "3".to_string(),
                },
                Permission {
                    access: Access::allow,
                    alias: format!("system_of_{}_access_", rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect::<String>()),
                    id: None,
                    level: PermissionLevel::object_type_field,
                    kind: PermissionKind::create,
                    name: "Access to birthday".to_string(),
                    object: "4".to_string(),
                },
            ],
            link: vec![],
            link_type: vec![
                Permission {
                    access: Access::allow,
                    alias: format!("system_of_{}_access_", rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect::<String>()),
                    id: None,
                    level: PermissionLevel::link_type,
                    kind: PermissionKind::create,
                    name: "Access to birthday".to_string(),
                    object: "*".to_string(),
                }, ],
        },
    };
    match block_on(Secure_Repository::createGroup(&g)) {
        Ok(i) => { g.id = Some(i); }
        Err(e) => {
            println!("{:?}", e);
            panic!("Error create group");
        }
    };
    vec![g]
}

pub fn add_test_user() -> User {
    let login = format!("user_login_{}", rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>());

    match block_on(User_Repository::getUserByLogin(login.clone())) {
        Ok(u) => { return u; }
        Err(_) => {}
    };

    let mut user = User {
        id: None,
        login,
        password: match bcrypt::hash("testestest".to_string(), DEFAULT_COST) {
            Ok(h) => { h }
            Err(e) => { panic!("Cannt hashed password {}", e); }
        },
        access_token: "".to_string(),
        oauth: "".to_string(),
        groups: get_user_groups(),
        date_last_active: None,
        date_registred: Utc::now().naive_utc(),
    };
    let res = block_on(User_Repository::createUser(&user));
    assert_eq!(res.is_ok(), true);
    user.id = Some(res.unwrap());
    return user;
}

pub fn login() -> User {
    let user = add_test_user();
    let client = Client::tracked(rocket_build()).expect("valid rocket instance");
    let mut response = client.post(uri!("/login")).body(format!("{{\
            \"login\":\"{}\",\
            \"password\":\"testestest\"\
        }}", user.login)).remote(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
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
    return user;
}

pub fn get_random_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>()
}

pub fn get_object_string_by_object_type_id(id: String) -> String {
    let mut ot = block_on(Object_Repository::getObjectTypeFromId(id)).unwrap();
    for field in ot.fields.iter_mut() {
        field.value = Some(match field.kind.as_str() {
            "varchar(255)" => { get_random_string() }
            "timestamp" => { format!("1988-03-02T02:00:00.00Z") }
            _ => {"".to_string()}
        });
    }
    let the_object = Object{
        filled: ot,
        date_created: Utc::now().naive_utc(),
        date_deleted: None,
        user_created: block_on(User_Repository::getUserById("1".to_string())).unwrap(),
        user_deleted: None,
        hash: "".to_string(),
        id: None
    };

    let mut value_object = to_value(the_object).unwrap();
    value_object["user_created_id"] = Value::String("1".to_string());
    let user_created_id = value_object.get_mut("user_created").unwrap();
    *user_created_id = Value::String("1".to_string());

    return value_object.to_string();

    format!("{{\
            \"filled\":{{\
                \"id\":\"1\",
                \"fields\":[
                    {{
                        \"id\":\"1\",
                        \"alias\":\"lastname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"lastname\",
                        \"value\":\"Platonov\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }},
                    {{
                        \"id\":\"2\",
                        \"alias\":\"firstname\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"firstname\",
                        \"value\":\"Alexander\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }},
                    {{
                        \"id\":\"3\",
                        \"alias\":\"patronymic\",
                        \"kind\":\"varchar(255)\",
                        \"name\":\"patronymic\",
                        \"value\":\"Alexanderovich\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }},
                    {{
                        \"id\":\"4\",
                        \"alias\":\"birthday\",
                        \"kind\":\"timestamp\",
                        \"name\":\"datetime\",
                        \"value\":\"1988-03-02T02:00:00.00Z\",
                        \"require\":true,
                        \"index\":true,
                        \"preview\":true
                    }}
                ],
                \"kind\":\"object\",
                \"alias\":\"fl\"
            }},\
            \"date_created\":\"1988-03-02T02:30:00.00Z\",\
            \"user_created\":\"1\",
            \"hash\":\"\"\
        }}")
}