use std::str::FromStr;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Error as Sqlx_Error;
use async_std::task::{block_on, JoinHandle, spawn};
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{create_table, get_insert, insert, select, sql, sql_one, update};
use crate::{cache_it, model, remove_it_from_cache};
use crate::model::error::RepositoryError;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    fn getFieldFromRow(row: PgRow) -> Field {
        Field {
            id: Some(row.get::<i64, &str>("id").to_string()),
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
        cache_it!(&alias,object_type_by_alias,{
                let kind_row = sql_one(format!("select kind,id from object_type where alias = '{}' limit 1", alias).as_str()).await?;
                let kind = kind_row.get::<String, &str>("kind").to_string();
                let kind_id = kind_row.get::<i64, &str>("id").to_string();
                let fields_rows = sql(format!("select * from field where object_type_id = '{}'", &kind_id).as_str()).await?;
                let id = Some(kind_id);
                let fields = Repository::getFieldsFromRows(fields_rows);
                ObjectType {
                    id,
                    fields,
                    kind,
                    alias,
                }
        })
    }

    pub async fn getObjectTypeFromObjectId(id: String) -> Result<ObjectType, RepositoryError> {
        let object_row = sql_one(format!("select object_type_id from object where id = '{}' limit 1", id).as_str()).await?;
        let object_type_id = object_row.get::<i64, &str>("object_type_id").to_string();
        let object_type = Self::getObjectTypeFromId(object_type_id).await?;

        Ok(object_type)
    }


    pub async fn hydrateFilledObjectType(id: String) -> Result<Object, RepositoryError> {
        cache_it!(&id,object_by_id,{
                let mut objectType = Self::getObjectTypeFromObjectId(id.clone()).await?;
            let row = sql_one(format!("select * from {} where id='{}'", objectType.alias.clone(), id.clone()).as_str()).await?;
            for field in &mut objectType.fields {
                field.value = Some(row.get::<String, &str>(field.alias.as_str()));
            }
            let object_row = sql_one(format!("select * from object where id = '{}' limit 1", id).as_str()).await?;

           Object {
                filled: objectType,
                date_created: object_row.get::<NaiveDateTime, &str>("date_created"),
                date_deleted: object_row.get::<Option<NaiveDateTime>, &str>("date_created"),
                user_created: model::user::repository::repository::Repository::getUserById(object_row.get::<String, &str>("user_created_id")).await?,
                user_deleted: match object_row.get::<Option<i64>, &str>("user_deleted_id") {
                    Some(v) => Some(model::user::repository::repository::Repository::getUserById(v.to_string()).await?),
                    None => None
                },
                hash: row.get::<String, &str>("hash"),
                id: Some(id),
            }
        })
    }

    pub fn objectToNameValues(the_object: &Object, id: String) -> Vec<(String, String)> {
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
        name_values
    }

    async fn insertObjectToTable(the_object: &Object, id: String) -> Result<String, RepositoryError> {
        let table = the_object.filled.alias.clone();
        let mut name_values = the_object.filled.fields
            .iter()
            .map(
                |f| (f.alias.clone(), match f.value.clone() {
                    Some(v) => format!("{}", v),
                    None => "".to_string()
                })
            )
            .collect::<Vec<_>>();
        name_values.push(("id".to_string(), id));
        Ok(insert(table, name_values).await?)
    }

    async fn insertObjectToGeneralTable(the_object: &Object) -> Result<String, RepositoryError> {
        Ok(insert("object", vec![
            // ("kind", the_object.filled.kind.as_str()),
            ("aggr_object_type_alias", the_object.filled.alias.as_str()),
            ("object_type_id", match the_object.filled.id.as_ref() {
                None => { return Err(RepositoryError { message: format!("ObjectType must has id") }); }
                Some(i) => { i.as_str() }
            }),
            ("user_created_id", match the_object.user_created.id.as_ref() {
                None => { return Err(RepositoryError { message: format!("User must has id") }); }
                Some(d) => { d }
            }.as_str()),
            ("date_created", the_object.date_created.to_string().as_str()),
        ]).await?)
    }

    pub async fn getObjectTypeFromId(id: String) -> Result<ObjectType, Sqlx_Error> {
        cache_it!(&id,object_type_by_id,{
            let fields_rows = sql(format!("select * from field where id = '{}'", id).as_str()).await?;
            let fields = Repository::getFieldsFromRows(fields_rows);
            let row = sql_one(format!("select kind from object_type where id = '{}' limit 1", id).as_str()).await?;

            let kind = row.get::<String, &str>("kind").to_string();
            let alias = row.get::<String, &str>("alias").to_string();

            ObjectType {
                id: Some(id),
                fields,
                kind,
                alias,
            }
        })
    }

    pub async fn createObject(the_object: &Object) -> Result<String, RepositoryError> {
        let id = Self::insertObjectToGeneralTable(the_object).await?;
        Self::insertObjectToTable(the_object, id.clone()).await?;
        Ok(id)
    }


    pub async fn updateObject(the_object: &Object) -> Result<String, RepositoryError> {
        let id = match the_object.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("Object must has id") }); }
            Some(i) => { i }
        };
        remove_it_from_cache!(id,object_by_id);
        let nv = Self::objectToNameValues(the_object, id.clone());
        let table = the_object.filled.alias.clone();
        Ok(update(table, nv, vec![("id".to_string(), "=".to_string(), id.clone())]).await?)
    }

    pub async fn deleteObject(id: &str, user: User) -> Result<(), RepositoryError> {
        let ids = id.to_string();
        remove_it_from_cache!(&ids,object_by_id);
        update("object", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted_id", match user.id {
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
                ("object_type_id".to_string(), id.to_string()),
            ])));
        }

        for future in futures {
            block_on(future)?;
        }
        Ok(())
    }


    pub async fn deleteObjectType(id: &str, user: User) -> Result<(), RepositoryError> {
        let ids = id.to_string();
        let ot = Self::getObjectTypeFromId(ids.clone()).await?;
        remove_it_from_cache!(&ids,object_type_by_id);
        remove_it_from_cache!(&ot.alias,object_type_by_alias);
        update("object_type", vec![
            ("date_deleted", Utc::now().to_rfc3339().as_str()),
            ("user_deleted_id", match user.id {
                None => { return Err(RepositoryError { message: format!("User must has id") }); }
                Some(i) => { i }
            }.as_str()),
        ], vec![("id", "=", id)]).await?;
        Ok(())
    }
}