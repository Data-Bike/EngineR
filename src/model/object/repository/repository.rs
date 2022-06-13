use std::collections::LinkedList;
use std::future::{Future};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::task::{Context, RawWaker, Waker};
use futures::task::noop_waker_ref;
use chrono::{DateTime, ParseResult, Utc};
use sqlx::Error as Sqlx_Error;
// use futures_util::async_await::poll;
use core::convert;
use std::borrow::Borrow;
use std::pin::Pin;
use std::ptr;
use async_std::prelude::FutureExt;
use async_std::task::{block_on, JoinHandle, spawn, spawn_blocking};
use async_std::task_local;
use rocket::futures;
// use core::future::Future;
use rocket::futures::poll;
use rocket::http::ext::IntoCollection;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{create_table, get_case, get_insert, insert, select, sql, sql_one, update};
use crate::model;
use crate::model::error::RepositoryError;
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
            id: Some(row.get::<String, &str>("id")),
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

    pub async fn getObjectTypeFromAlias(alias: String) -> Result<ObjectType, RepositoryError> {
        let fields_rows = sql(format!("select * from field where alias = '{}'", alias).as_str()).await?;

        let fields = Repository::getFieldsFromRows(fields_rows);

        let kind_row = sql_one(format!("select kind from object_type where alias = '{}' limit 1", alias).as_str()).await?;
        let kind = kind_row.get::<String, &str>("kind").to_string();
        let id = Some(kind_row.get::<String, &str>("id").to_string());
        Ok(ObjectType {
            id,
            fields,
            kind,
            alias,
        })
    }

    pub async fn getObjectTypeFromObjectId(id: String) -> Result<ObjectType, RepositoryError> {
        let fields_rows = sql(format!("select f.* from object o join field f on f.alias=o.alias where o.id = '{}'", id).as_str()).await?;
        let fields = Repository::getFieldsFromRows(fields_rows);

        let kind_alias_row = sql_one(format!("select * from object where id = '{}' limit 1", id).as_str()).await?;
        let kind_alias = (kind_alias_row.get::<String, &str>("kind").to_string(), kind_alias_row.get::<String, &str>("alias").to_string());
        let id = Some(kind_alias_row.get::<String, &str>("id").to_string());

        Ok(ObjectType {
            id,
            fields,
            kind: kind_alias.0,
            alias: kind_alias.1,
        })
    }


    pub async fn hydrateFilledObjectType(id: String) -> Result<Object, RepositoryError> {
        let mut objectType = Self::getObjectTypeFromObjectId(id.clone()).await?;
        let row = sql_one(format!("select * from {} where id='{}'", objectType.alias.clone(), id.clone()).as_str()).await?;
        for field in &mut objectType.fields {
            field.value = Some(row.get::<String, &str>(field.alias.as_str()));
        }
        let object_row = sql_one(format!("select * from object where id = '{}' limit 1", id).as_str()).await?;

        Ok(Object {
            filled: objectType,
            date_created: match DateTime::<Utc>::from_str(object_row.get::<&str, &str>("date_created")) {
                Ok(d) => { d }
                Err(e) => { return Err(RepositoryError { message: format!("Cannot parse date:{}", e.to_string()) }); }
            },
            date_deleted: match object_row.get::<Option<&str>, &str>("date_created") {
                Some(v) => Some(match DateTime::<Utc>::from_str(v) {
                    Ok(d) => { d }
                    Err(e) => { return Err(RepositoryError { message: format!("Cannot parse date:{}", e.to_string()) }); }
                }),
                None => None
            },
            user_created: model::user::repository::repository::Repository::getUserById(object_row.get::<String, &str>("date_created")).await?,
            user_deleted: match object_row.get::<Option<String>, &str>("user_deleted") {
                Some(v) => Some(model::user::repository::repository::Repository::getUserById(v).await?),
                None => None
            },
            hash: row.get::<String, &str>("hash"),
            id: Some(id),
        })
    }

    async fn insertObjectToTable(the_object: &Object, id: String) -> Result<String, RepositoryError> {
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
        Ok(insert(table, name_values).await?)
    }

    async fn insertObjectToGeneralTable(the_object: &Object) -> Result<String, RepositoryError> {
        Ok(insert("object", vec![
            ("kind", the_object.filled.kind.as_str()),
            ("alias", the_object.filled.alias.as_str()),
            ("user_created", match the_object.user_created.id.as_ref() {
                None => { return Err(RepositoryError { message: format!("User must has id") }); }
                Some(d) => { d }
            }.as_str()),
            ("date_created", the_object.date_created.to_rfc3339().as_str()),
        ]).await?)
    }

    pub async fn getObjectTypeFromId(id: String) -> Result<ObjectType, Sqlx_Error> {
        let fields_rows = sql(format!("select * from field where id = '{}'", id).as_str()).await?;
        let fields = Repository::getFieldsFromRows(fields_rows);
        let row = sql_one(format!("select kind from object_type where id = '{}' limit 1", id).as_str()).await?;

        let kind = row.get::<String, &str>("kind").to_string();
        let alias = row.get::<String, &str>("alias").to_string();

        Ok(ObjectType {
            id: Some(id),
            fields,
            kind,
            alias,
        })
    }

    pub async fn createObject(the_object: &Object) -> Result<String, RepositoryError> {
        let id = Self::insertObjectToGeneralTable(the_object).await?;
        Self::insertObjectToTable(the_object, id.clone()).await?;
        Ok(id)
    }

    pub async fn deleteObject(id: &str, user: User) -> Result<(), RepositoryError> {
        update("object", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted", match user.id {
                None => { return Err(RepositoryError { message: format!("User must has id") }); }
                Some(i) => { i }
            }.as_str()),
        ], vec![("id", "=", id)]).await?;
        Ok(())
    }

    pub async fn searchObject(the_object: &Object) -> Result<Vec<Object>, RepositoryError> {
        let case = the_object
            .filled
            .fields
            .iter()
            .filter(|f| f.value.is_some())
            .map(|f| (f.alias.clone(), "=".to_string(), f.value.clone().unwrap_or("".to_string())))
            .collect::<Vec<(String, String, String)>>();

        let table = the_object.filled.alias.clone();

        select(table, vec!["id".to_string()], vec![case])
            .await?
            .iter()
            .filter_map(|o| o.try_get::<String, &str>("id").ok())
            .map(|id| block_on(Self::hydrateFilledObjectType(id)))
            .collect::<Result<Vec<Object>, RepositoryError>>()
    }

    pub async fn createObjectType(object_type: ObjectType) -> Result<(), RepositoryError> {
        let fields = object_type
            .fields
            .iter()
            .map(|f| (f.alias.clone(), f.kind.clone()))
            .collect::<Vec<_>>();

        let id = insert("object_type", vec![
            ("alias", object_type.alias.as_str()),
            ("kind", object_type.kind.as_str()),
        ]).await?;

        let mut futures: Vec<JoinHandle<_>> = vec![];

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
        Ok(())
    }

    pub async fn deleteObjectType(id: &str, user: User) -> Result<(), RepositoryError> {
        update("object_type", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted", match user.id {
                None => { return Err(RepositoryError { message: format!("User must has id") }); }
                Some(i) => { i }
            }.as_str()),
        ], vec![("id", "=", id)]).await;
        Ok(())
    }
}