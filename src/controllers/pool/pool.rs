use std::ops::Deref;
use async_std::task::block_on;
use lazy_static::lazy_static;
use sqlx::{Pool, Postgres, Row};
use sqlx::postgres::{PgPoolOptions, PgRow};
// extern crate lazy_static;
lazy_static! {
 static ref  pool: Pool<Postgres> = block_on(PgPoolOptions::new()
    .max_connections(5)
    .connect("postgres://postgres:password@localhost/test")
).expect("Failed to connect to database");
}




pub async fn sql(sql: &str) -> Vec<PgRow> {

    match sqlx::query(sql).fetch_all(pool.deref()).await {
        Ok(rows) => rows,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}

pub async fn sql_one(sql: &str) -> PgRow {
    match sqlx::query(sql).fetch_one(pool.deref()).await {
        Ok(row) => row,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}