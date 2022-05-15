use std::ops::Deref;
use async_std::task::block_on;
use lazy_static::lazy_static;
use sqlx::{Pool, Postgres, Row};
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

pub fn get_insert(table: String, name_values: Vec<(String, String)>) -> String {
    let mut names = "".to_string();
    let mut values = "".to_string();
    let mut name = "".to_string();
    let mut value = "".to_string();
    for row in name_values {
        name = row.0;
        value = row.1;
        names = format!("{},{}", name, names);
        values = format!("{},{}", value, values);
    }
    names.pop();
    values.pop();
    format!("insert into {} ({}) values ({})", table, names, values)
}

pub fn get_update(table: String, name_values: Vec<(String, String)>, cases: Vec<(String, String, String)>) -> String {
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

pub fn get_case(cases: &Vec<(String, String, String)>) -> String {
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

pub async fn insert(table: String, name_values: Vec<(String, String)>) -> String {
    let row = sql_one(get_insert(table, name_values).as_str()).await;
    row.get::<String, &str>("id")
}


pub async fn update(table: String, name_values: Vec<(String, String)>, cases: Vec<(String, String, String)>) -> u64 {
    exec(get_update(table, name_values, cases).as_str()).await.rows_affected()
}

pub async fn select(table: String, names: Vec<String>, where_cases: Vec<Vec<(String, String, String)>>) -> Vec<PgRow> {
    sql(get_select(table, names, where_cases).as_str()).await
}

