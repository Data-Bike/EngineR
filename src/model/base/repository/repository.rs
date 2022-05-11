use std::collections::LinkedList;
use std::fmt::format;
use chrono::DateTime;
use postgres::{Client, Error, NoTls, Row, RowIter};
use rocket::futures::future::err;
use crate::model::link::entity::link::Link;
use crate::model::object::entity::object::{Field, Object, ObjectType};
use crate::model::user::repository::repository;

pub struct Repository {
    pub db: &'static mut Client,
}

impl Repository {
    pub fn new(db: &'static mut Client) -> Self {
        Repository { db }
    }
    pub fn getObjectById(&mut self, id: String) {
        let sql = format!("select * from object where id='{}'",id);

        let obj = match self.db.query_one(&sql, &[]) {
            Ok(row) => {
                Object{
                    filled: ObjectType {
                        fields: vec![],
                        kind: "".to_string(),
                        alias: "".to_string()
                    },
                    hash: "".to_string()
                }
            }
            Err(error) => {
                panic!("{}",error.to_string().as_str())
            }
        };
    }
}