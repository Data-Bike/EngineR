use std::collections::LinkedList;
use std::future::{Future};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::task::{Context, RawWaker, Waker};
use futures::task::noop_waker_ref;
use chrono::{DateTime, Utc};
// use futures_util::async_await::poll;
use core::convert;
use std::pin::Pin;
use std::ptr;
use async_std::prelude::FutureExt;
use async_std::task::{block_on, JoinHandle, spawn, spawn_blocking};
use async_std::task_local;
use rocket::futures;
// use core::future::Future;
use rocket::futures::poll;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{create_table, get_insert, insert, sql, sql_one, update};
use crate::model;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::secure::entity::permission::PermissionLevel::object;
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    fn getFieldFromRow(row: PgRow) -> Field {
        Field {
            alias: row.get::<String, &str>("alias"),
            kind: row.get::<String, &str>("kind"),
            name: row.get::<String, &str>("name"),
            default: if row.get::<String, &str>("kind") == "" { None } else { Some(row.get::<String, &str>("kind").to_string()) },
            value: None,
            require: row.get::<bool, &str>("require"),
            index: row.get::<bool, &str>("index"),
            preview: row.get::<bool, &str>("preview"),
        }
    }

    fn getFieldsFromRows(rows: Vec<PgRow>) -> Vec<Field> {
        let mut res = Vec::<Field>::new();
        for row in rows {
            res.push(Repository::getFieldFromRow(row));
        }
        res
    }

    pub async fn getObjectTypeFromAlias(alias: String) -> ObjectType {
        let fields_rows = sql(format!("select * from field where alias = '{}'", alias).as_str()).await;

        let fields = Repository::getFieldsFromRows(fields_rows);

        let kind_row = sql_one(format!("select kind from object_type where alias = '{}' limit 1", alias).as_str()).await;
        let kind = kind_row.get::<String, &str>("kind").to_string();
        ObjectType {
            fields,
            kind,
            alias,
        }
    }

    pub async fn getObjectTypeFromObjectId(id: String) -> ObjectType {
        let fields_rows = sql(format!("select f.* from object o join field f on f.alias=o.alias where o.id = '{}'", id).as_str()).await;
        let fields = Repository::getFieldsFromRows(fields_rows);

        let kind_alias_row = sql_one(format!("select * from object where id = '{}' limit 1", id).as_str()).await;
        let kind_alias = (kind_alias_row.get::<String, &str>("kind").to_string(), kind_alias_row.get::<String, &str>("alias").to_string());


        ObjectType {
            fields,
            kind: kind_alias.0,
            alias: kind_alias.1,
        }
    }


    pub async fn hydrateFilledObjectType(id: String) -> Object {
        let mut objectType = Self::getObjectTypeFromObjectId(id.clone()).await;
        let row = sql_one(format!("select * from {} where id='{}'", objectType.alias.clone(), id.clone()).as_str()).await;
        for field in &mut objectType.fields {
            field.value = Some(row.get::<String, &str>(field.alias.as_str()));
        }
        let object_row = sql_one(format!("select * from object where id = '{}' limit 1", id).as_str()).await;

        Object {
            filled: objectType,
            date_created: DateTime::<Utc>::from_str(object_row.get::<&str, &str>("date_created")).unwrap(),
            date_deleted: match object_row.get::<Option<&str>, &str>("date_created") {
                Some(v) => Some(DateTime::<Utc>::from_str(v).unwrap()),
                None => None
            },
            user_created: model::user::repository::repository::Repository::getUserById(object_row.get::<String, &str>("date_created")).await,
            user_deleted: match object_row.get::<Option<String>, &str>("user_deleted") {
                Some(v) => Some(model::user::repository::repository::Repository::getUserById(v).await),
                None => None
            },
            hash: row.get::<String, &str>("hash"),
        }
    }

    async fn insertObjectToTable(the_object: &Object, id: String) -> String {
        let table = the_object.filled.alias.clone();
        let mut name_values = the_object.filled.fields
            .iter()
            .map(
                |f| (f.alias.clone(), match f.value.clone() {
                    Some(v) => format!("'{}'", v),
                    None => "null".to_string()
                })
            )
            .collect::<Vec<_>>();
        name_values.push(("id".to_string(), id));
        insert(table, name_values).await
    }

    async fn insertObjectToGeneralTable(the_object: &Object) -> String {
        insert("object", vec![
            ("kind", the_object.filled.kind.as_str()),
            ("alias", the_object.filled.alias.as_str()),
            ("user_created", the_object.user_created.id.as_str()),
            ("date_created", the_object.date_created.to_rfc3339().as_str()),
        ]).await
    }

    pub async fn createObject(the_object: &Object) -> String {
        let id = Self::insertObjectToGeneralTable(the_object).await;
        Self::insertObjectToTable(the_object, id.clone()).await;
        id
    }

    pub async fn deleteObject(id: &str, user: User) {
        update("object", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted", user.id.as_str()),
        ], vec![("id", "=", id)]).await;
    }

    pub async fn createObjectType(object_type: ObjectType) {
        let fields = object_type
            .fields
            .iter()
            .map(|f| (f.alias.clone(), f.kind.clone()))
            .collect::<Vec<_>>();

        let id = insert("object_type", vec![
            ("alias", object_type.alias.as_str()),
            ("kind", object_type.kind.as_str()),
        ]).await;

        let mut futures: Vec<JoinHandle<String>> = vec![];

        futures.push(spawn(create_table(object_type.alias.clone(), fields)));
        for field in object_type.fields.iter() {
            futures.push(spawn(insert("field".to_string(), vec![
                ("alias".to_string(), field.alias.clone()),
                ("name".to_string(), field.name.clone()),
                ("kind".to_string(), field.kind.clone()),
                ("default".to_string(), field.default.clone().unwrap_or("".to_string())),
                ("require".to_string(), if field.require { "1".to_string() } else { "0".to_string() }),
                ("index".to_string(), if field.index { "1".to_string() } else { "0".to_string() }),
                ("preview".to_string(), if field.preview { "1".to_string() } else { "0".to_string() }),
                ("object_type".to_string(), id.to_string()),
            ])));
        }

        for future in futures {
            block_on(future);
        }
    }

    pub async fn deleteObjectType(id: &str, user: User) {
        update("object_type", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted", user.id.as_str()),
        ], vec![("id", "=", id)]).await;
    }
}