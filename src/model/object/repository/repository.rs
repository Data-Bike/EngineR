use std::collections::LinkedList;
use chrono::DateTime;
use rocket::futures::future::err;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{sql, sql_one};
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
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

    async fn getObjectTypeFromAlias(&mut self, alias: String) -> ObjectType {
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

    async fn getObjectTypeFromObjectId(&mut self, id: String) -> ObjectType {
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


    async fn hydrateFilledObjectType(&mut self, alias: String, id: String) -> Object {
        let mut objectType = self.getObjectTypeFromObjectId(id.clone()).await;
        let row = sql_one(format!("select * from {} where id='{}'", alias, id.clone()).as_str()).await;
        for field in &mut objectType.fields {
            field.value = Some(row.get::<String, &str>(field.alias.as_str()).to_string());
        }
        Object {
            filled: objectType,
            hash: row.get::<String, &str>("hash").to_string(),
        }
    }
}