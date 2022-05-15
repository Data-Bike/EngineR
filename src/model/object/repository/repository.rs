use std::collections::LinkedList;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use rocket::futures::future::err;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{insert, sql, sql_one};
use crate::model;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;
use crate::model::user::repository::repository;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    fn getFieldFromRow(row: PgRow) -> Field {
        Field {
            alias: row.get::<String, &str>("alias").to_string(),
            kind: row.get::<String, &str>("kind").to_string(),
            name: row.get::<String, &str>("name").to_string(),
            default: if row.get::<String, &str>("kind") == "" { None } else { Some(row.get::<String, &str>("kind").to_string()) },
            value: None,
            require: row.get::<String, &str>("require").to_string() == "1",
            index: row.get::<String, &str>("index").to_string() == "1",
            preview: row.get::<String, &str>("preview").to_string() == "1",
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
        let fields_rows = sql(format!("select * from fields where table = '{}'", alias).as_str()).await;

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
        let fields_rows = sql(format!("select f.* from object o join fields f on f.table=o.alias where o.id = '{}'", id).as_str()).await;
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
            field.value = Some(row.get::<String, &str>(field.alias.as_str()).to_string());
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
            hash: row.get::<String, &str>("hash").to_string(),
        }
    }

    async fn insertObjectToTable(object: &Object) -> String {
        let table = object.filled.alias.clone();
        let name_values = object.filled.fields
            .iter()
            .map(
                |f| (f.alias.clone(), match f.value.clone() {
                    Some(v) => format!("'{}'", v),
                    None => "null".to_string()
                })
            )
            .collect::<Vec<_>>();
        insert(table, name_values).await
    }

    async fn insertObjectToGeneralTable(object: &Object, user: &User) -> String {
        insert("object".to_string(), vec![
            ("kind".to_string(), object.filled.kind.clone()),
            ("alias".to_string(), object.filled.alias.clone()),
            ("user_created".to_string(), "".to_string()),
            ("date_created".to_string(), "".to_string()),
        ]).await
    }

    pub async fn createObject(object: &Object) -> String {
        "".to_string()
    }
}