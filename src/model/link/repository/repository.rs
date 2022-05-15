use std::collections::LinkedList;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use rocket::futures::future::err;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::repository::repository;

use crate::controllers::pool::pool;
use crate::controllers::pool::pool::{sql, sql_one};
use crate::model;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    fn getFieldFromRow(row: PgRow) -> Field {
        Field {
            alias: row.get::<String, &str>("alias").to_string(),
            kind: row.get::<String, &str>("kind").to_string(),
            default: if row.get::<String, &str>("kind") == "" { None } else { Some(row.get::<String, &str>("kind").to_string()) },
            value: None,
            require: row.get::<String, &str>("require").to_string() == "true",
            index: row.get::<String, &str>("index").to_string() == "true",
            preview: row.get::<String, &str>("preview").to_string() == "true",
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
        let row = sql_one(format!("select kind from object_type where alias = '{}' limit 1", alias).as_str()).await;

        let kind = row.get::<String, &str>("kind").to_string();

        ObjectType {
            fields,
            kind,
            alias,
        }
    }

    pub async fn getObjectTypeFromObjectId(id: String) -> ObjectType {
        let fields_rows = sql(format!("select f.* from object o join fields f on f.table=o.alias where o.id = '{}'", id).as_str()).await;
        let fields = Repository::getFieldsFromRows(fields_rows);

        let kind_alias_row = sql_one(format!("select kind, alias from object where id = '{}' limit 1", id).as_str()).await;
        let kind_alias = (kind_alias_row.get::<String, &str>("kind").to_string(), kind_alias_row.get::<String, &str>("alias").to_string());

        ObjectType {
            fields,
            kind: kind_alias.0,
            alias: kind_alias.1,
        }
    }

    pub async fn getLinkRowsByToId(id: String, limit: u64, skip: u64) -> Vec<PgRow> {
        sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id2 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await
    }

    pub async fn getLinkRowsByFromId(id: String, limit: u64, skip: u64) -> Vec<PgRow> {
        sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await
    }

    async fn getLinkRowsByAssignId(id: String, limit: u64, skip: u64) -> Vec<PgRow> {
        sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' or id2 = '{}' limit {} offset {}", &id, &id, limit, skip).as_str()).await
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

    pub async fn getEnityFromRow(&'static mut self, row: PgRow) -> Link {
        let object_from = Self::hydrateFilledObjectType(
            row.get::<String, &str>("object_from").to_string(),
        ).await;

        let object_to = Self::hydrateFilledObjectType(
            row.get::<String, &str>("object_to").to_string(),
        ).await;

        let user_created = repository::Repository::getUserById(row.get::<String, &str>("user_created").to_string()).await;
        let user_deleted = if row.get::<String, &str>("user_deleted").as_str() != "" { Some(repository::Repository::    getUserById(row.get::<String, &str>("user_deleted")).await) } else { None };
        let date_created = DateTime::parse_from_rfc3339(row.get::<String, &str>("date_created").as_str()).unwrap();
        let date_deleted = if row.get::<String, &str>("date_deleted").as_str() != "" { Some(DateTime::parse_from_rfc3339(row.get::<String, &str>("date_deleted").as_str()).unwrap()) } else { None };

        Link {
            object_from,
            object_to,
            user_created,
            user_deleted,
            date_created,
            date_deleted,
        }
    }

    pub async fn setLink(id1: String, id2: String, userName: String) {
        let sql = format!("insert into links (id1,id2,user_created,date_created) values ({},{},{},{})",
                          id1, id2, userName, chrono::offset::Utc::now().to_rfc3339());
        sql_one(sql.as_str()).await;
    }

    pub async fn unsetLink(id: String, userName: String) {
        let sql = format!("update links set user_deleted = '{}', date_deleted = '{}' where id = '{}'",
                          userName, chrono::offset::Utc::now().to_rfc3339(), id);
        sql_one(sql.as_str()).await;
    }
}