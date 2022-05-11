use std::collections::LinkedList;
use chrono::DateTime;
use rocket::futures::future::err;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::repository::repository;

use crate::controllers::pool::pool;
use crate::controllers::pool::pool::{sql, sql_one};

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

    pub async fn hydrateFilledObjectType(alias: String, id: String) -> Object {
        let mut objectType = Self::getObjectTypeFromObjectId(id.clone()).await;

        let row = sql_one(format!("select * from {} where id='{}'", alias, id.clone()).as_str()).await;

        for field in &mut objectType.fields {
            field.value = Some(row.get::<String, &str>(field.alias.as_str()).to_string());
        }
        Object {
            filled: objectType,
            hash: row.get::<String, &str>("hash").to_string(),
        }
    }

    pub async fn getEnityFromRow(&'static mut self, row: PgRow) -> Link {
        let from = Self::hydrateFilledObjectType(
            row.get::<String, &str>("from_alias").to_string(),
            row.get::<String, &str>("from_id").to_string(),
        ).await;

        let to = Self::hydrateFilledObjectType(
            row.get::<String, &str>("to_alias").to_string(),
            row.get::<String, &str>("to_id").to_string(),
        ).await;

        let mut userRep = repository::Repository::new();
        let userLinked = userRep.getUserById(row.get::<String, &str>("userLinkedId").to_string()).await;
        let userUnlinked = if row.get::<String, &str>("userUnlinkedId").as_str() != "" { Some(userRep.getUserById(row.get::<String, &str>("userUnlinkedId")).await) } else { None };
        let dateLinked = DateTime::parse_from_rfc3339(row.get::<String, &str>("dateLinked").as_str()).unwrap();
        let dateUnlinked = if row.get::<String, &str>("dateUnlinked").as_str() != "" { Some(DateTime::parse_from_rfc3339(row.get::<String, &str>("dateUnlinked").as_str()).unwrap()) } else { None };

        Link {
            from,
            to,
            userLinked,
            userUnlinked,
            dateLinked,
            dateUnlinked,
        }
    }

    pub async fn setLink(id1: String, id2: String, userName: String) {
        let sql = format!("insert into links (id1,id2,userLinked,dateLinked) values ({},{},{},{})",
                          id1, id2, userName, chrono::offset::Utc::now().to_rfc3339());
        sql_one(sql.as_str()).await;
    }

    pub async fn unsetLink(id: String, userName: String) {
        let sql = format!("update links set userUnlinked = '{}', dateUnlinked = '{}' where id = '{}'",
                          userName, chrono::offset::Utc::now().to_rfc3339(), id);
        sql_one(sql.as_str()).await;
    }
}