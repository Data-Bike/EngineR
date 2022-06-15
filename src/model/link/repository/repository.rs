use caches::Cache;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::cache_it;
use crate::model::link::entity::link::{Link, LinkType};
use crate::model::user::repository::repository;
use crate::model::object::repository::repository::Repository as Object_repository;
use crate::controllers::pool::pool::{sql, sql_one};
use crate::model::error::RepositoryError;
use crate::model::lfu_cache::cache::CACHE;
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }


    pub async fn getLinkRowsByToId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
        Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id2 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await?)
    }

    pub async fn getLinkRowsByFromId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
        Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await?)
    }

    async fn getLinkRowsByAssignId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
        Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' or id2 = '{}' limit {} offset {}", &id, &id, limit, skip).as_str()).await?)
    }

    pub async fn getLinkTypeById(id: &str) -> Result<LinkType, RepositoryError> {
        let ids = id.to_string();
        cache_it!(&ids,link_type_by_id,{
            let row = sql_one(format!("select * from link_type where id='{}'", id).as_str()).await?;
                LinkType {
                    id: Some(id.to_string()),
                    alias: row.get::<String, &str>("alias"),
                    name: row.get::<String, &str>("name"),
                    object_type_from: Object_repository::getObjectTypeFromId(row.get::<String, &str>("object_type_from_id")).await?,
                    object_type_to: Object_repository::getObjectTypeFromId(row.get::<String, &str>("object_type_to_id")).await?,
                }
        })
    }

    pub async fn getEnityFromRow(&'static mut self, row: PgRow) -> Result<Link, RepositoryError> {
        let object_from = Object_repository::hydrateFilledObjectType(
            row.get::<String, &str>("object_from").to_string(),
        ).await?;

        let object_to = Object_repository::hydrateFilledObjectType(
            row.get::<String, &str>("object_to").to_string(),
        ).await?;

        let user_created = repository::Repository::getUserById(row.get::<String, &str>("user_created").to_string()).await?;
        let user_deleted = if row.get::<String, &str>("user_deleted").as_str() != "" { Some(repository::Repository::getUserById(row.get::<String, &str>("user_deleted")).await?) } else { None };
        let date_created = DateTime::<Utc>::from(match DateTime::parse_from_rfc3339(row.get::<String, &str>("date_created").as_str()) {
            Ok(d) => { d }
            Err(e) => { return Err(RepositoryError { message: format!("Cannot parse rfc3339 date:{}", e.to_string()) }); }
        });
        let date_deleted = if row.get::<String, &str>("date_deleted").as_str() != "" {
            Some(DateTime::<Utc>::from(match DateTime::parse_from_rfc3339(row.get::<String, &str>("date_deleted").as_str()) {
                Ok(d) => { d }
                Err(e) => { return Err(RepositoryError { message: format!("Cannot parse rfc3339 date:{}", e.to_string()) }); }
            }))
        } else { None };
        let link_type_id = row.get::<&str, &str>("link_type_id");
        let link_type = Self::getLinkTypeById(link_type_id).await?;
        let id = Some(row.get::<String, &str>("id"));
        Ok(Link {
            id,
            object_from,
            object_to,
            link_type,
            user_created,
            user_deleted,
            date_created,
            date_deleted,
        })
    }

    pub async fn setLink(id1: String, id2: String, userName: String) -> Result<(), RepositoryError> {
        let sql = format!("insert into link (object_from,object_to,user_created,date_created) values ({},{},{},{})",
                          id1, id2, userName, chrono::offset::Utc::now().to_rfc3339());
        sql_one(sql.as_str()).await?;
        Ok(())
    }

    pub async fn unsetLink(id: String, userName: String) -> Result<(), RepositoryError> {
        let sql = format!("update link set user_deleted = '{}', date_deleted = '{}' where id = '{}'",
                          userName, chrono::offset::Utc::now().to_rfc3339(), id);
        sql_one(sql.as_str()).await?;
        Ok(())
    }
}