use std::str::FromStr;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Error as Sqlx_Error;
use async_std::task::{block_on, JoinHandle, spawn};
use sqlx::postgres::PgRow;
use sqlx::Row;
use crate::controllers::pool::pool::{create_table, get_insert, insert, select, select_one, sql, sql_one, update};
use crate::{cache_it, model, remove_it_from_cache};
use crate::model::dictionary::entity::dictionary::{Dictionary, DictionaryType};
use crate::model::error::RepositoryError;
use crate::model::lfu_cache::cache::CACHE;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::entity::user::User;

pub struct Repository {}

impl Repository {
    pub fn new() -> Self {
        Repository {}
    }

    fn getDictionaryFromRow(row: &PgRow) -> Dictionary {
        Dictionary {
            id: Some(row.get::<i64, &str>("id").to_string()),
            alias: row.get::<String, &str>("alias"),
            name: row.get::<String, &str>("name"),
        }
    }

    fn getDictionariesFromRows(row: Vec<PgRow>) -> Vec<Dictionary> {
        row.iter().map(|d| Self::getDictionaryFromRow(d)).collect()
    }

    pub async fn getDictionaryTypeFromAlias(alias: String) -> Result<DictionaryType, RepositoryError> {
        cache_it!(&alias,dictionary_type_by_alias,{
                let res = select_one(
                    "dictionary_type".to_string(),
                    vec!["*".to_string()],
                    vec![vec![("alias".to_string(),"=".to_string(),alias)]]
                ).await?;

                let id = res.get::<i64,&str>("id").to_string();
                let alias = res.get::<String,&str>("alias").to_string();
                let name = res.get::<String,&str>("name").to_string();
                let dictionaries = Self::getDictionariesByTypeId(id.clone()).await?;

                DictionaryType {
                    id:Some(id),
                    alias,
                    name,
                    dictionaries
                }
        })
    }

    pub async fn getDictionaryTypeById(id: String) -> Result<DictionaryType, RepositoryError> {
        cache_it!(&id,dictionary_type_by_id,{
                let res = select_one(
                    "dictionary_type".to_string(),
                    vec!["*".to_string()],
                    vec![vec![("id".to_string(),"=".to_string(),id.clone())]]
                ).await?;

                let alias = res.get::<String,&str>("alias").to_string();
                let name = res.get::<String,&str>("name").to_string();
                let dictionaries = Self::getDictionariesByTypeId(id.clone()).await?;

                DictionaryType {
                    id:Some(id),
                    alias,
                    name,
                    dictionaries
                }
        })
    }

    pub async fn getDictionaryById(id: String) -> Result<Dictionary, RepositoryError> {
        cache_it!(&id,dictionary_by_id,{
                let res = select_one(
                    "dictionary".to_string(),
                    vec!["*".to_string()],
                    vec![vec![("id".to_string(),"=".to_string(),id)]]
                ).await?;

                let id = Some(res.get::<i64,&str>("id").to_string());
                let alias = res.get::<String,&str>("alias").to_string();
                let name = res.get::<String,&str>("name").to_string();
                let type_id = res.get::<i64,&str>("type_id").to_string();

                Dictionary {
                    id,
                    alias,
                    name
                }
        })
    }

    pub async fn getDictionariesByTypeId(type_id: String) -> Result<Vec<Dictionary>, RepositoryError> {
        let res = select(
            "dictionary".to_string(),
            vec!["*".to_string()],
            vec![vec![("type_id".to_string(), "=".to_string(), type_id)]],
        ).await?;

        Ok(Self::getDictionariesFromRows(res))
    }

    pub async fn getDictionaryByAlias(alias: String) -> Result<Dictionary, RepositoryError> {
        cache_it!(&alias,dictionary_by_alias,{
                let res = select_one(
                    "dictionary".to_string(),
                    vec!["*".to_string()],
                    vec![vec![("alias".to_string(),"=".to_string(),alias)]]
                ).await?;

                let id = Some(res.get::<i64,&str>("id").to_string());
                let alias = res.get::<String,&str>("alias").to_string();
                let name = res.get::<String,&str>("name").to_string();

                Dictionary {
                    id,
                    alias,
                    name
                }
        })
    }


    pub fn dictionaryTypeToNameValues(the_dictionary_type: &DictionaryType) -> Vec<(String, String)> {
        let mut name_values = vec![
            ("alias".to_string(), the_dictionary_type.alias.to_string()),
            ("name".to_string(), the_dictionary_type.name.to_string()),
        ];
        match the_dictionary_type.id.as_ref() {
            None => {}
            Some(i) => { name_values.push(("id".to_string(), i.to_string())); }
        }

        name_values
    }

    pub fn dictionaryToNameValues(the_dictionary: &Dictionary, type_id: Option<String>) -> Vec<(String, String)> {
        let mut name_values = vec![
            ("alias".to_string(), the_dictionary.alias.to_string()),
            ("name".to_string(), the_dictionary.name.to_string()),
        ];
        match the_dictionary.id.as_ref() {
            None => {}
            Some(i) => { name_values.push(("id".to_string(), i.to_string())); }
        };
        match type_id.as_ref() {
            None => {}
            Some(i) => { name_values.push(("type_id".to_string(), i.to_string())); }
        };

        name_values
    }

    pub async fn createDictionaryType(the_dictionary_type: &DictionaryType) -> Result<String, RepositoryError> {
        let name_values = Self::dictionaryTypeToNameValues(the_dictionary_type);
        Ok(insert("dictionary_type".to_string(), name_values).await?)
    }

    pub async fn createDictionary(the_dictionary: &Dictionary, type_id: String) -> Result<String, RepositoryError> {
        let name_values = Self::dictionaryToNameValues(the_dictionary, Some(type_id));
        Ok(insert("dictionary".to_string(), name_values).await?)
    }


    pub async fn updateDictionaryType(the_dictionary_type: &DictionaryType) -> Result<String, RepositoryError> {
        let id = match the_dictionary_type.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("DictionaryType must has id") }); }
            Some(i) => { i }
        };
        remove_it_from_cache!(id,dictionary_type_by_id);
        remove_it_from_cache!(&the_dictionary_type.alias,dictionary_type_by_alias);
        let nv = Self::dictionaryTypeToNameValues(the_dictionary_type);
        Ok(update("dictionary_type".to_string(), nv, vec![("id".to_string(), "=".to_string(), id.clone())]).await?)
    }

    pub async fn updateDictionary(the_dictionary: &Dictionary) -> Result<String, RepositoryError> {
        let id = match the_dictionary.id.as_ref() {
            None => { return Err(RepositoryError { message: format!("Dictionary must has id") }); }
            Some(i) => { i }
        };
        remove_it_from_cache!(id,dictionary_by_id);
        remove_it_from_cache!(&the_dictionary.alias,dictionary_by_alias);
        let nv = Self::dictionaryToNameValues(the_dictionary, None);
        Ok(update("dictionary".to_string(), nv, vec![("id".to_string(), "=".to_string(), id.clone())]).await?)
    }
}