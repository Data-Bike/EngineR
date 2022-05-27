use std::future::{Future};
use std::ops::Deref;
use async_std::task::block_on;
use lazy_static::lazy_static;
use postgres::types::ToSql;
use sqlx::{Acquire, Connection, Error, Executor, Pool, Postgres, Row, Transaction};
use sqlx::postgres::{PgPoolOptions, PgQueryResult, PgRow};

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

pub async fn exec(sql: &str) -> PgQueryResult {
    match sqlx::query(sql).execute(pool.deref()).await {
        Ok(row) => row,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}

pub async fn sql_tr(sql: &str, tr: &mut Transaction<'_, Postgres>) -> Vec<PgRow> {
    match sqlx::query(sql).fetch_all(&mut *tr).await {
        Ok(rows) => rows,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}

pub async fn sql_one_tr(sql: &str, tr: &mut Transaction<'_, Postgres>) -> PgRow {
    match sqlx::query(sql).fetch_one(&mut *tr).await {
        Ok(row) => row,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}

pub async fn exec_tr(sql: &str, tr: &mut Transaction<'_, Postgres>) -> PgQueryResult {
    match sqlx::query(sql).execute(&mut *tr).await {
        Ok(row) => row,
        Err(e) => panic!("{}", e.to_string().as_str())
    }
}

pub fn get_insert<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>) -> String {
    let mut names = "".to_string();
    let mut values = "".to_string();
    let mut name = "".to_string();
    let mut value = "".to_string();
    for row in name_values {
        name = row.0.to_string();
        value = row.1.to_string();
        names = format!("{},{}", name, names);
        values = format!("{},{}", value, values);
    }
    names.pop();
    values.pop();
    format!("insert into {} ({}) values ({})", table, names, values)
}

pub fn get_update<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>, cases: Vec<(T, T, T)>) -> String {
    let mut key_values = "".to_string();

    for name_value in name_values {
        key_values = format!("{} {}={}, ", key_values, name_value.0, name_value.1);
    }
    key_values.pop();
    key_values.pop();

    let where_case = get_case(&cases);
    format!("update {} set {} where {}", table, key_values, where_case)
}

pub fn get_select(table: String, names: Vec<String>, where_cases: Vec<Vec<(String, String, String)>>) -> String {
    let names_string = names.join(", ");
    let where_string = get_where(&where_cases);
    format!("select {} from {} where {}", names_string, table, where_string)
}

pub fn get_case<T: ToString + std::fmt::Display>(cases: &Vec<(T, T, T)>) -> String {
    cases
        .iter()
        .map(|c| format!("{} {} {}", c.0, c.1, c.2))
        .collect::<Vec<_>>()
        .join(" and ")
}

pub fn get_where(where_cases: &Vec<Vec<(String, String, String)>>) -> String {
    where_cases
        .iter()
        .map(|c| format!("({})", get_case(c)))
        .collect::<Vec<_>>()
        .join(" or ")
}

pub fn get_create_table(table: String, fields: Vec<(String, String)>) -> String {
    let mut fields_str = fields.iter().map(|f| format!("{} {}", f.0, f.1)).collect::<Vec<_>>().join(",");

    format!("CREATE TABLE {} (id serial PRIMARY KEY,{},FOREIGN KEY (user_id)
      REFERENCES object (id))", table, fields_str)
}

pub fn get_alter_table(table: String, fields_to_alter: Vec<(String, String, String)>) -> String {
    let alter_str = fields_to_alter
        .iter()
        .map(|f| format!("{} COLUMN {} TYPE {}", f.0, f.1, f.2))
        .collect::<Vec<_>>()
        .join(",");
    format!("ALTER TABLE {} {}", table, alter_str)
}

pub async fn insert<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>) -> String {
    let row = sql_one(get_insert(table, name_values).as_str()).await;
    row.get::<String, &str>("id")
}


pub async fn update<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>, cases: Vec<(T, T, T)>) -> u64 {
    exec(get_update(table, name_values, cases).as_str()).await.rows_affected()
}

pub async fn select(table: String, names: Vec<String>, where_cases: Vec<Vec<(String, String, String)>>) -> Vec<PgRow> {
    sql(get_select(table, names, where_cases).as_str()).await
}

pub async fn create_table(table: String, fields: Vec<(String, String)>) -> String {
    exec(get_create_table(table, fields).as_str()).await.rows_affected().to_string()
}

pub async fn alter_table(table: String, fields_to_alter: Vec<(String, String, String)>) -> String {
    exec(get_alter_table(table, fields_to_alter).as_str()).await.rows_affected().to_string()
}


pub async fn insert_tr<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>, tr: &mut Transaction<'_, Postgres>) -> String {
    let row = sql_one_tr(get_insert(table, name_values).as_str(), tr).await;
    row.get::<String, &str>("id")
}


pub async fn update_tr<T: ToString + std::fmt::Display>(table: T, name_values: Vec<(T, T)>, cases: Vec<(T, T, T)>, tr: &mut Transaction<'_, Postgres>) -> u64 {
    exec_tr(get_update(table, name_values, cases).as_str(), tr).await.rows_affected()
}

pub async fn select_tr(table: String, names: Vec<String>, where_cases: Vec<Vec<(String, String, String)>>, tr: &mut Transaction<'_, Postgres>) -> Vec<PgRow> {
    sql_tr(get_select(table, names, where_cases).as_str(), tr).await
}

pub async fn create_table_tr(table: String, fields: Vec<(String, String)>, tr: &mut Transaction<'_, Postgres>) -> String {
    exec_tr(get_create_table(table, fields).as_str(), tr).await.rows_affected().to_string()
}

pub async fn alter_table_tr(table: String, fields_to_alter: Vec<(String, String, String)>, tr: &mut Transaction<'_, Postgres>) -> String {
    exec_tr(get_alter_table(table, fields_to_alter).as_str(), tr).await.rows_affected().to_string()
}

// TODO: transactions
pub async fn tr() -> Result<Transaction<'static, Postgres>, Error> {
    pool.deref().begin().await
}