use caches::Cache;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::executor::block_on;
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::cache_it;
use crate::model::link::entity::link::{Link, LinkType};
use crate::model::user::repository::repository;
use crate::model::object::repository::repository::Repository as Object_repository;
use crate::model::app::pool::pool::{insert, select, sql, sql_one};
use crate::model::error::RepositoryError;
use crate::model::lfu_cache::cache::CACHE;
use crate::model::secure::entity::permission::PermissionLevel::link;
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }


    // pub async fn getLinkRowsByToId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
    //     Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id2 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await?)
    // }
    //
    // pub async fn getLinkRowsByFromId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
    //     Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' limit {} offset {}", &id, limit, skip).as_str()).await?)
    // }
    //
    // async fn getLinkRowsByAssignId(id: String, limit: u64, skip: u64) -> Result<Vec<PgRow>, RepositoryError> {
    //     Ok(sql(format!("select o1.kind as 'from_kind', o1.alias as 'from_alias', o1.id as 'from_id', o2.kind as 'to_kind', o2.alias as 'to_alias', o2.id as 'to_id', l.* from links l join object o1 on l.id1=o1.id join object o2 on l.id2=02.id where id1 = '{}' or id2 = '{}' limit {} offset {}", &id, &id, limit, skip).as_str()).await?)
    // }

    pub async fn getLinkTypeById(id: &str) -> Result<LinkType, RepositoryError> {
        let ids = id.to_string();
        cache_it!(&ids,link_type_by_id,{
            let row = sql_one(format!("select * from link_type where id='{}'", id).as_str()).await?;
                LinkType {
                    id: Some(id.to_string()),
                    alias: row.get::<String, &str>("alias"),
                    name: row.get::<String, &str>("name"),
                    object_type_from: Object_repository::getObjectTypeFromId(row.get::<i64, &str>("object_type_from_id").to_string()).await?,
                    object_type_to: Object_repository::getObjectTypeFromId(row.get::<i64, &str>("object_type_to_id").to_string()).await?,
                }
        })
    }

    pub fn linkTypeToNameValues(link_type: &LinkType) -> Vec<(String, String)> {
        let mut res: Vec<(String, String)> = vec![];
        match link_type.object_type_from.id.as_ref() {
            Some(x) => { res.push(("object_type_from_id".to_string(), x.to_string())) }
            None => {}
        }
        match link_type.object_type_to.id.as_ref() {
            Some(x) => { res.push(("object_type_to_id".to_string(), x.to_string())) }
            None => {}
        }
        match link_type.id.as_ref() {
            Some(x) => { res.push(("id".to_string(), x.to_string())) }
            None => {}
        }
        res.push(("name".to_string(), link_type.name.clone()));
        res.push(("alias".to_string(), link_type.alias.clone()));

        res
    }

    pub async fn createLinkType(link_type: &LinkType) -> Result<LinkType, RepositoryError> {
        let nv = Self::linkTypeToNameValues(link_type);
        let id = insert("link_type".to_string(), nv).await?;
        let mut lt = link_type.clone();
        lt.id = Some(id);
        Ok(lt)
    }

    pub async fn getEnityFromRow(row: &PgRow) -> Result<Link, RepositoryError> {
        let object_from = Object_repository::hydrateFilledObjectType(
            row.get::<i64, &str>("object_from_id").to_string(),
        ).await?;

        let object_to = Object_repository::hydrateFilledObjectType(
            row.get::<i64, &str>("object_to_id").to_string(),
        ).await?;

        let user_created = repository::Repository::getUserById(row.get::<i64, &str>("user_created_id").to_string()).await?;
        let user_deleted =  row.get::<Option<i64>, &str>("user_deleted_id").and_then(|d| Some(block_on(repository::Repository::getUserById(d.to_string())))).transpose()?;
        // { Some(repository::Repository::getUserById(row.get::<i64, &str>("user_deleted_id").to_string()).await?) } else { None };
        let date_created = row.get::<NaiveDateTime, &str>("date_created");
        let date_deleted = row.get::<Option<NaiveDateTime>, &str>("date_deleted");
        let link_type_id = row.get::<i64, &str>("link_type_id").to_string();
        let link_type = Self::getLinkTypeById(link_type_id.as_str()).await?;
        let id = Some(row.get::<i64, &str>("id").to_string());
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

    pub async fn getLinksFromRows(rows: Vec<PgRow>) -> Result<Vec<Link>, RepositoryError> {
        Ok(rows
            .iter()
            .map(|r| block_on(Self::getEnityFromRow(r)))
            .collect::<Result<Vec<Link>, RepositoryError>>()?)
    }

    pub async fn getLinkById(id: String) -> Result<Link, RepositoryError> {
        let rs = select(
            "link".to_string(),
            vec!["*".to_string()],
            vec![vec![("id".to_string(), "=".to_string(), id)]],
        ).await?;
        let lnk = match Self::getLinksFromRows(rs).await?.pop() {
            None => { return Err(RepositoryError { message: format!("Does not have any link") }); }
            Some(l) => { l }
        };
        Ok(lnk)
    }

    pub async fn setLink(the_link: Link) -> Result<(), RepositoryError> {
        let id1 = match the_link.object_from.id {
            None => { return Err(RepositoryError { message: format!("Object_from must has id") }); }
            Some(i) => { i }
        };

        let id2 = match the_link.object_to.id {
            None => { return Err(RepositoryError { message: format!("Object_to must has id") }); }
            Some(i) => { i }
        };

        let userId = match the_link.user_created.id {
            None => { return Err(RepositoryError { message: format!("User_created must has id") }); }
            Some(i) => { i }
        };

        let linkTypeId = match the_link.link_type.id {
            None => { return Err(RepositoryError { message: format!("Link_type must has id") }); }
            Some(i) => { i }
        };

        let sql = format!("insert into \"link\" (\"object_from_id\",\"object_to_id\",\"user_created_id\",\"date_created\",\"link_type_id\") values ('{}','{}','{}','{}','{}')",
                          id1, id2, userId, chrono::offset::Utc::now().to_rfc3339(),linkTypeId);
        sql_one(sql.as_str()).await?;
        Ok(())
    }

    pub async fn unsetLink(the_link: Link) -> Result<(), RepositoryError> {
        let id = match the_link.id {
            None => { return Err(RepositoryError { message: format!("Link must has id") }); }
            Some(i) => { i }
        };

        let userId = match the_link.user_deleted.and_then(|u| u.id) {
            None => { return Err(RepositoryError { message: format!("User_deleted must has id") }); }
            Some(i) => { i }
        };

        let sql = format!("update link set user_deleted = '{}', date_deleted = '{}' where id = '{}'",
                          userId, chrono::offset::Utc::now().to_rfc3339(), id);
        sql_one(sql.as_str()).await?;
        Ok(())
    }
}